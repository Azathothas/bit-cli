use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{Error, Result, session::CheckedIncomingConnection, stream_connect::ConnectionKind};
use buffers::{ByteBuf, ByteBufOwned};
use futures::TryFutureExt;
use librqbit_core::{
    hash_id::Id20,
    lengths::{ChunkInfo, ValidPieceIndex},
    peer_id::try_decode_peer_id,
};
use parking_lot::RwLock;
use peer_binary_protocol::{
    Handshake, MAX_MSG_LEN, Message,
    extended::{
        ExtendedMessage, PeerExtendedMessageIds, handshake::ExtendedHandshake,
        ut_metadata::UtMetadata, ut_pex::UtPex,
    },
    serialize_piece_preamble,
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use tokio::time::timeout;
use tracing::{Instrument, debug, trace, trace_span};

use crate::{
    read_buf::ReadBuf,
    spawn_utils::BlockingSpawner,
    stream_connect::StreamConnector,
    stream_transform::OutgoingTransform,
    type_aliases::{BoxAsyncReadVectored, BoxAsyncWrite},
};

/// BEP 6's two byte stand-ins for a bitfield.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HaveShortcut {
    /// Every piece.
    All,
    /// No piece.
    None,
}

pub trait PeerConnectionHandler {
    fn on_connected(&self, _connection_time: Duration) {}
    fn should_send_bitfield(&self) -> bool;

    /// BEP 6: whether this end's bitfield is all ones or all zeroes.
    ///
    /// Only consulted when the fast extension was negotiated, and it is what
    /// replaces a bitfield of a million bits with two bytes. `None` means send
    /// the bitfield. The default answers `None`, which is what every
    /// implementor did before this method existed.
    fn have_shortcut(&self) -> Option<HaveShortcut> {
        None
    }

    /// Whether the fast extension was negotiated on this connection.
    ///
    /// Set from the peer's handshake before the first message goes out. An
    /// implementor that does not care leaves the default, which does nothing.
    fn on_fast_negotiated(&self, _negotiated: bool) {}
    /// Serialize our bitfield into `buf`, resizing it as needed, and return
    /// its length. `buf` is owned by the caller and is not the shared
    /// fixed-size message buffer: a bitfield is one bit per piece, so only the
    /// implementor knows how large it has to be.
    fn serialize_bitfield_message_to_buf(&self, buf: &mut Vec<u8>) -> anyhow::Result<usize>;
    fn on_handshake(&self, handshake: Handshake, ckind: ConnectionKind) -> anyhow::Result<()>;

    /// The largest message this peer may send us, in bytes.
    ///
    /// The read buffer starts at `BUFLEN` and grows to this. It exists because
    /// one message is not bounded by any constant: a bitfield is one bit per
    /// piece, so a torrent past 262,104 pieces sends one that does not fit in
    /// 32,768 bytes and the connection died on it whatever either end did.
    /// See `TODO/peers.md` T-195.
    ///
    /// The implementor answers from the **torrent**, never from what the peer
    /// says it is about to send, because the second is a number a hostile peer
    /// picks. The default is the buffer that already exists, which is the
    /// behaviour every implementor had before this method.
    fn max_incoming_message_len(&self) -> usize {
        crate::read_buf::BUFLEN
    }
    fn on_extended_handshake(
        &self,
        extended_handshake: &ExtendedHandshake<ByteBuf>,
    ) -> anyhow::Result<()>;
    async fn on_received_message(&self, msg: Message<'_>) -> anyhow::Result<()>;
    fn should_transmit_have(&self, id: ValidPieceIndex) -> bool;
    fn on_uploaded_bytes(&self, bytes: u32);
    fn read_chunk(&self, chunk: &ChunkInfo, buf: &mut [u8]) -> anyhow::Result<()>;
    fn update_my_extended_handshake(
        &self,
        _handshake: &mut ExtendedHandshake<ByteBuf>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    fn client_name_and_version(&self) -> &str {
        crate::client_name_and_version()
    }
}

#[derive(Debug)]
pub enum WriterRequest {
    Message(Message<'static>),
    UtMetadata(UtMetadata<ByteBufOwned>),
    UtPex(UtPex<ByteBufOwned>),
    ReadChunkRequest(ChunkInfo),
    Disconnect(anyhow::Result<()>),
}

#[serde_as]
#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct PeerConnectionOptions {
    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    pub connect_timeout: Option<Duration>,

    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    pub read_write_timeout: Option<Duration>,

    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    pub keep_alive_interval: Option<Duration>,
}

pub(crate) struct PeerConnection<H> {
    handler: H,
    addr: SocketAddr,
    info_hash: Id20,
    peer_id: Id20,
    options: PeerConnectionOptions,
    spawner: BlockingSpawner,
    connector: Arc<StreamConnector>,
}

#[cfg(not(feature = "miri"))]
pub(crate) async fn with_timeout<T>(
    name: &'static str,
    timeout_value: Duration,
    fut: impl std::future::Future<Output = Result<T>>,
) -> crate::Result<T> {
    match timeout(timeout_value, fut).await {
        Ok(v) => v,
        Err(_) => Err(Error::Timeout(name)),
    }
}

#[cfg(feature = "miri")]
pub(crate) async fn with_timeout<T>(
    _name: &'static str,
    _timeout_value: Duration,
    fut: impl std::future::Future<Output = Result<T>>,
) -> crate::Result<T> {
    fut.await
}

struct ManagePeerArgs {
    handshake_supports_extended: bool,
    /// BEP 6: both ends set the reserved bit. `Handshake::new` sets ours on
    /// every connection, so this is the peer's answer.
    fast_negotiated: bool,
    read_buf: ReadBuf,
    write_buf: Box<[u8; MAX_MSG_LEN]>,
    read: BoxAsyncReadVectored,
    write: BoxAsyncWrite,
    outgoing_chan: tokio::sync::mpsc::UnboundedReceiver<WriterRequest>,
    have_broadcast: tokio::sync::broadcast::Receiver<ValidPieceIndex>,
}

impl<H: PeerConnectionHandler> PeerConnection<H> {
    pub fn new(
        addr: SocketAddr,
        info_hash: Id20,
        peer_id: Id20,
        handler: H,
        options: Option<PeerConnectionOptions>,
        spawner: BlockingSpawner,
        connector: Arc<StreamConnector>,
    ) -> Self {
        PeerConnection {
            handler,
            addr,
            info_hash,
            peer_id,
            spawner,
            options: options.unwrap_or_default(),
            connector,
        }
    }

    // By the time this is called:
    // read_buf should start with valuable data. The handshake should be removed from it.
    pub async fn manage_peer_incoming(
        &self,
        outgoing_chan: tokio::sync::mpsc::UnboundedReceiver<WriterRequest>,
        mut incoming: CheckedIncomingConnection,
        have_broadcast: tokio::sync::broadcast::Receiver<ValidPieceIndex>,
    ) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let rwtimeout = self
            .options
            .read_write_timeout
            .unwrap_or_else(|| Duration::from_secs(10));

        if incoming.handshake.info_hash != self.info_hash {
            return Err(Error::WrongInfoHash);
        }

        if incoming.handshake.peer_id == self.peer_id {
            return Err(Error::ConnectingToOurselves);
        }

        trace!(
            target: "librqbit::handshake",
            "incoming connection: id={:?}",
            try_decode_peer_id(incoming.handshake.peer_id)
        );

        let mut write_buf = Box::new([0u8; MAX_MSG_LEN]);
        let ours = Handshake::new(self.info_hash, self.peer_id);
        let hlen = ours.serialize_unchecked_len(&mut *write_buf);
        with_timeout(
            "writing handshake",
            rwtimeout,
            incoming
                .writer
                .write_all(&write_buf[..hlen])
                .map_err(Error::WriteHandshake),
        )
        .await?;

        // The peer's handshake, not ours. Both of these read `handshake` until
        // 2026-08-22, which was the one just built to send, so every incoming
        // peer was recorded under this session's own peer id and was assumed
        // to speak BEP 10 because `Handshake::new` always sets that bit. The
        // outgoing path a few lines below reads the peer's, which is what says
        // this was a slip rather than a design. See TODO/peers.md T-210.
        let handshake_supports_extended = incoming.handshake.supports_extended();
        let fast_negotiated = incoming.handshake.supports_fast();
        self.handler.on_fast_negotiated(fast_negotiated);

        self.handler
            .on_handshake(incoming.handshake, incoming.kind)
            .map_err(Error::Anyhow)?;

        // The session read the handshake into this buffer before it knew which
        // torrent the connection was for, so the bound is raised here, where
        // the handler is known.
        let mut read_buf = incoming.read_buf;
        read_buf.set_max_len(self.handler.max_incoming_message_len());

        self.manage_peer(ManagePeerArgs {
            handshake_supports_extended,
            fast_negotiated,
            read_buf,
            write_buf,
            read: incoming.reader,
            write: incoming.writer,
            outgoing_chan,
            have_broadcast,
        })
        .await
    }

    pub async fn manage_peer_outgoing(
        &self,
        outgoing_chan: tokio::sync::mpsc::UnboundedReceiver<WriterRequest>,
        have_broadcast: tokio::sync::broadcast::Receiver<ValidPieceIndex>,
    ) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        let rwtimeout = self
            .options
            .read_write_timeout
            .unwrap_or_else(|| Duration::from_secs(10));

        let connect_timeout = self
            .options
            .connect_timeout
            .unwrap_or_else(|| Duration::from_secs(10));

        let now = Instant::now();
        let (ckind, mut read, mut write) = with_timeout(
            "connecting",
            connect_timeout,
            self.connector.connect(self.addr),
        )
        .await?;

        // Added by this fork. The session's transform, if it has one, gets the
        // connection before the handshake is written. A transform that
        // negotiated something this peer does not speak has spent the
        // connection finding out, so it may ask for one more, made without it.
        // See stream_transform and TODO/peers.md T-163.
        if let Some(transform) = self.connector.stream_transform() {
            let transformed = with_timeout(
                "transforming outgoing connection",
                rwtimeout,
                transform
                    .outgoing(self.addr, self.info_hash, read, write)
                    .map_err(Error::Anyhow),
            )
            .await?;
            match transformed {
                OutgoingTransform::Stream(r, w) => {
                    read = r;
                    write = w;
                }
                OutgoingTransform::RetryPlaintext => {
                    let (_, r, w) = with_timeout(
                        "connecting",
                        connect_timeout,
                        self.connector.connect(self.addr),
                    )
                    .await?;
                    read = r;
                    write = w;
                }
            }
        }

        async move {
            self.handler.on_connected(now.elapsed());

            let mut write_buf = Box::new([0u8; MAX_MSG_LEN]);
            let handshake = Handshake::new(self.info_hash, self.peer_id);
            let hsz = handshake.serialize_unchecked_len(&mut *write_buf);
            with_timeout(
                "writing",
                rwtimeout,
                write
                    .write_all(&write_buf[..hsz])
                    .map_err(Error::WriteHandshake),
            )
            .await?;

            let mut read_buf = ReadBuf::new();
            read_buf.set_max_len(self.handler.max_incoming_message_len());
            let h = read_buf.read_handshake(&mut read, rwtimeout).await?;
            let handshake_supports_extended = h.supports_extended();
            let fast_negotiated = h.supports_fast();
            self.handler.on_fast_negotiated(fast_negotiated);
            trace!(
                target: "librqbit::handshake",
                peer_id=?h.peer_id,
                decoded_id=?try_decode_peer_id(h.peer_id),
                supports_extended=handshake_supports_extended,
                supports_fast=fast_negotiated,
                "connected",
            );
            if h.info_hash != self.info_hash {
                return Err(Error::WrongInfoHash);
            }

            if h.peer_id == self.peer_id {
                return Err(Error::ConnectingToOurselves);
            }

            self.handler.on_handshake(h, ckind).map_err(Error::Anyhow)?;

            self.manage_peer(ManagePeerArgs {
                handshake_supports_extended,
                fast_negotiated,
                read_buf,
                write_buf,
                read,
                write,
                outgoing_chan,
                have_broadcast,
            })
            .await
        }
        .instrument(trace_span!("", kind=%ckind))
        .await
    }

    async fn manage_peer(&self, args: ManagePeerArgs) -> Result<()> {
        let ManagePeerArgs {
            handshake_supports_extended,
            fast_negotiated,
            mut read_buf,
            mut write_buf,
            mut read,
            mut write,
            mut outgoing_chan,
            mut have_broadcast,
        } = args;

        use tokio::io::AsyncWriteExt;

        let rwtimeout = self
            .options
            .read_write_timeout
            .unwrap_or_else(|| Duration::from_secs(10));

        let extended_handshake: RwLock<Option<PeerExtendedMessageIds>> = RwLock::new(None);
        let extended_handshake_ref = &extended_handshake;
        let supports_extended = handshake_supports_extended;

        if supports_extended {
            let mut my_extended = ExtendedHandshake::new();
            my_extended.v = Some(ByteBuf(self.handler.client_name_and_version().as_bytes()));
            my_extended.yourip = Some(self.addr.ip().into());
            self.handler
                .update_my_extended_handshake(&mut my_extended)
                .map_err(Error::Anyhow)?;
            let my_extended = Message::Extended(ExtendedMessage::Handshake(my_extended));
            trace!(target: "librqbit::handshake", "sending extended handshake: {:?}", &my_extended);
            let esz = my_extended.serialize(&mut *write_buf, &Default::default)?;
            with_timeout(
                "writing extended handshake",
                rwtimeout,
                write.write_all(&write_buf[..esz]).map_err(Error::Write),
            )
            .await?;
        }

        let writer = async move {
            let keep_alive_interval = self
                .options
                .keep_alive_interval
                .unwrap_or_else(|| Duration::from_secs(120));

            // BEP 6 changes what the first message may be, and makes sending
            // one mandatory: a peer that negotiated the fast extension expects
            // a bitfield, a have-all or a have-none before anything else, and
            // sending nothing is a protocol violation rather than an
            // omission. Without the extension this stays exactly as it was,
            // including sending nothing when there is nothing to announce.
            let shortcut = fast_negotiated
                .then(|| self.handler.have_shortcut())
                .flatten();
            match shortcut {
                Some(shortcut) => {
                    let msg = match shortcut {
                        HaveShortcut::All => Message::HaveAll,
                        HaveShortcut::None => Message::HaveNone,
                    };
                    let len = msg.serialize(&mut *write_buf, &Default::default)?;
                    with_timeout(
                        "writing the bitfield shortcut",
                        rwtimeout,
                        write.write_all(&write_buf[..len]).map_err(Error::Write),
                    )
                    .await?;
                    trace!(?shortcut, "sent the bitfield shortcut");
                }
                None if self.handler.should_send_bitfield() => {
                    // Not write_buf: that one is MAX_MSG_LEN, which bounds every
                    // message whose size the protocol bounds. A bitfield is one
                    // bit per piece and is bounded by the torrent instead, so it
                    // gets a buffer sized for this torrent. It is written once per
                    // connection and dropped here.
                    let mut bitfield_buf = Vec::new();
                    let len = self
                        .handler
                        .serialize_bitfield_message_to_buf(&mut bitfield_buf)
                        .map_err(Error::Anyhow)?;
                    with_timeout(
                        "writing bitfield",
                        rwtimeout,
                        write.write_all(&bitfield_buf[..len]).map_err(Error::Write),
                    )
                    .await?;
                    trace!("sent bitfield");
                }
                None if fast_negotiated => {
                    let len = Message::HaveNone.serialize(&mut *write_buf, &Default::default)?;
                    with_timeout(
                        "writing have none",
                        rwtimeout,
                        write.write_all(&write_buf[..len]).map_err(Error::Write),
                    )
                    .await?;
                    trace!("sent have none in place of no bitfield at all");
                }
                None => {}
            }

            let len = Message::Unchoke.serialize(&mut *write_buf, &Default::default)?;
            with_timeout(
                "writing",
                rwtimeout,
                write.write_all(&write_buf[..len]).map_err(Error::Write),
            )
            .await?;
            trace!("sent unchoke");

            let mut broadcast_closed = false;

            loop {
                let req = loop {
                    break tokio::select! {
                        r = have_broadcast.recv(), if !broadcast_closed => match r {
                            Ok(id) => {
                                if self.handler.should_transmit_have(id) {
                                     WriterRequest::Message(Message::Have(id.get()))
                                } else {
                                    continue
                                }
                            },
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                broadcast_closed = true;
                                debug!("broadcast channel closed, will not poll it anymore");
                                continue
                            },
                            _ => continue
                        },
                        r = timeout(keep_alive_interval, outgoing_chan.recv()) => match r {
                            Ok(Some(msg)) => msg,
                            Ok(None) => {
                                return Err(Error::TorrentIsNotLive);
                            }
                            Err(_) => WriterRequest::Message(Message::KeepAlive),
                        }
                    };
                };

                tokio::task::yield_now().await;

                let mut uploaded_add = None;

                trace!("about to send: {:?}", &req);
                let ext_msg_ids = &|| {
                    extended_handshake_ref
                        .read()
                        .as_ref()
                        .map(|e| *e)
                        .unwrap_or_default()
                };

                let len = match req {
                    WriterRequest::Message(msg) => msg.serialize(&mut *write_buf, ext_msg_ids)?,
                    WriterRequest::UtMetadata(utm) => {
                        Message::Extended(ExtendedMessage::UtMetadata(utm.as_borrowed()))
                            .serialize(&mut *write_buf, ext_msg_ids)?
                    }
                    WriterRequest::UtPex(ut_pex) => {
                        Message::Extended(ExtendedMessage::UtPex(ut_pex.as_borrowed()))
                            .serialize(&mut *write_buf, ext_msg_ids)?
                    }
                    WriterRequest::ReadChunkRequest(chunk) => {
                        #[allow(unused_mut)]
                        let mut skip_reading_for_e2e_tests = false;

                        #[cfg(test)]
                        {
                            use tracing::warn;
                            // This is poor-mans fault injection for running e2e tests.
                            use crate::tests::test_util::TestPeerMetadata;
                            let tpm = TestPeerMetadata::from_peer_id(self.peer_id);
                            use rand::RngExt;
                            if rand::rng().random_bool(tpm.disconnect_probability()) {
                                return Err(Error::TestDisconnect);
                            }

                            #[allow(clippy::cast_possible_truncation)]
                            let sleep_ms = (rand::rng().random::<f64>()
                                * (tpm.max_random_sleep_ms as f64))
                                as u64;
                            if sleep_ms > 0 {
                                tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                            }

                            if rand::rng().random_bool(tpm.bad_data_probability()) {
                                warn!(
                                    "will NOT actually read the data to simulate a malicious peer that sends garbage"
                                );
                                write_buf.fill(0);
                                skip_reading_for_e2e_tests = true;
                            }
                        }

                        // this whole section is an optimization
                        let preamble_len = serialize_piece_preamble(&chunk, &mut *write_buf);
                        let full_len = preamble_len + chunk.size as usize;
                        if !skip_reading_for_e2e_tests {
                            self.spawner
                                .block_in_place_with_semaphore(|| {
                                    self.handler
                                        .read_chunk(&chunk, &mut write_buf[preamble_len..full_len])
                                })
                                .await
                                .map_err(Error::ReadChunk)?;
                        }

                        uploaded_add = Some(chunk.size);
                        full_len
                    }
                    WriterRequest::Disconnect(res) => {
                        trace!("disconnect requested, closing writer");
                        match res {
                            Ok(()) => return Err(Error::Disconnect),
                            Err(e) => return Err(Error::DisconnectWithSource(e)),
                        }
                    }
                };

                with_timeout(
                    "writing",
                    rwtimeout,
                    write.write_all(&write_buf[..len]).map_err(Error::Write),
                )
                .await?;

                if let Some(uploaded_add) = uploaded_add {
                    self.handler.on_uploaded_bytes(uploaded_add)
                }
            }

            // For type inference.
            #[allow(unreachable_code)]
            Ok::<_, Error>(())
        };

        let reader = async move {
            loop {
                let message = read_buf.read_message(&mut read, rwtimeout).await?;
                trace!("received: {:?}", &message);

                tokio::task::yield_now().await;

                if let Message::Extended(ExtendedMessage::Handshake(h)) = &message {
                    *extended_handshake_ref.write() = Some(h.peer_extended_messages());
                    self.handler
                        .on_extended_handshake(h)
                        .map_err(Error::Anyhow)?;
                } else {
                    self.handler
                        .on_received_message(message)
                        .await
                        .map_err(Error::Anyhow)?;
                }
            }

            // For type inference.
            #[allow(unreachable_code)]
            Ok::<_, Error>(())
        };

        tokio::select! {
            r = reader => {
                if let Err(e) = r.as_ref() {
                    trace!("reader finished with error: {e:#}");
                } else {
                    trace!("reader finished without error");
                }
                r
            }
            r = writer => {
                if let Err(e) = r.as_ref() {
                    trace!("writer finished with error: {e:#}");
                } else {
                    trace!("writer finished without error");
                }
                r
            }
        }
    }
}
