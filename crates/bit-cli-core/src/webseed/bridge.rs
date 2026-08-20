//! A loopback BitTorrent peer backed by an HTTP source.
//!
//! `librqbit` has no notion of an HTTP source: its only entry point for
//! torrent data is the peer protocol. So a web seed is presented to the
//! session as an ordinary peer. The bridge dials the session's own listen
//! port, announces the pieces its source can serve, unchokes, and answers each
//! `request` with bytes fetched over ranged GETs.
//!
//! Nothing here verifies piece hashes. Fetched bytes reach the session as
//! normal peer blocks, so the session's own verification applies and a source
//! serving wrong data is dropped exactly like a lying peer.
//!
//! Two things separate this from a naive "claim everything" bridge:
//!
//! - The announced bitfield carries only the pieces the source's scope covers
//!   **in full**. A source holding half a piece cannot satisfy that piece's
//!   hash on its own, so claiming it would make the session request bytes the
//!   bridge has to refuse.
//! - When that is not the whole torrent, the bridge advertises BEP 21
//!   `upload_only`. A partial seed that does not say so reads to the session
//!   as a leecher that happens to be missing pieces.
//!
//! The bridge only ever seeds. It never sends `interested`, so it cannot
//! consume the session's upload bandwidth.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use librqbit::ByteBuf;
use librqbit_core::Id20;
use librqbit_core::peer_id::generate_peer_id;
use librqbit_peer_protocol::{Handshake, Message, Piece};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinSet;

use crate::layout::Layout;
use crate::webseed::fetch::{FetchError, Fetcher};

/// Azureus-style client prefix for a bridge's peer id.
///
/// It has to differ from the session's own id, or the session drops the
/// connection as a self-connect.
const PEER_ID_PREFIX: &[u8; 8] = b"-BCws01-";

/// Serialized keep-alive: a bare zero length prefix.
const KEEP_ALIVE: [u8; 4] = [0, 0, 0, 0];

/// Wire size of a BitTorrent v1 handshake.
const HANDSHAKE_LEN: usize = 68;

/// Bytes a message needs beyond its variable-length payload: the length
/// prefix, the message id, and a `piece` message's index and offset.
const MESSAGE_OVERHEAD: usize = 13;

/// How often to send a keep-alive. `librqbit` drops a peer that is silent for
/// longer than its read timeout, which defaults to ten seconds.
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(5);

/// Largest block the bridge serves. Real clients ask for 16 KiB; far above
/// that is a malformed request.
const MAX_REQUEST_LEN: u32 = 128 * 1024;

/// Longest frame accepted from the session, which bounds the read buffer.
const MAX_FRAME_LEN: usize = 1024 * 1024;

/// Requests the bridge tells the session it will keep queued.
const REQUEST_QUEUE: u32 = 250;

/// First delay before reconnecting. Doubles per failure.
const RECONNECT_BASE: Duration = Duration::from_secs(1);

/// Longest delay between reconnection attempts.
const RECONNECT_MAX: Duration = Duration::from_secs(30);

/// How many loopback ports a bridge remembers having connected from.
const MAX_REMEMBERED_PORTS: usize = 64;

/// BitTorrent message id for the BEP 10 extension protocol.
const MSGID_EXTENDED: u8 = 20;

/// Extension message id 0 is the extended handshake.
const EXTENDED_HANDSHAKE: u8 = 0;

/// What a bridge is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeState {
    /// Dialling the session, or waiting to retry.
    Connecting,
    /// Connected, unchoked, and serving.
    Active,
    /// Nothing to do: the torrent is not live, or it is already complete.
    Idle,
    /// Out for now, and coming back. The source spent its error budget and
    /// `--web-seed-cooldown` is non-zero, so the bridge is sleeping until the
    /// deadline and will reconnect. A cooling source is not a failed one: a
    /// caller waiting on it should keep waiting. See `TODO/multi-source.md`,
    /// T-137.
    Cooling,
    /// The source is unusable and the bridge has given up on it.
    Failed,
}

/// Live state of one bridge, readable while it runs.
#[derive(Debug)]
pub struct BridgeStatus {
    state: Mutex<BridgeState>,
    error: Mutex<Option<String>>,
    served_bytes: AtomicU64,
    blocks: AtomicU64,
    local_port: AtomicU16,
    /// Every loopback port this bridge has connected from.
    ports: Mutex<Vec<u16>>,
    /// Blocks the session has asked for and not yet been given.
    ///
    /// This is the session's request window seen from the other end, and it
    /// is the number that says whether the pipeline is deep enough to keep
    /// the link busy. `bench leech` samples it.
    in_flight: AtomicU64,
    peak_in_flight: AtomicU64,
    requests: AtomicU64,
    /// Total time from a request arriving to its block going out, over every
    /// block served. Divided by [`Self::blocks`] it is the mean service time,
    /// which with the depth above bounds throughput at depth over service
    /// time.
    service_nanos: AtomicU64,
}

impl Default for BridgeStatus {
    fn default() -> Self {
        Self {
            state: Mutex::new(BridgeState::Connecting),
            error: Mutex::new(None),
            served_bytes: AtomicU64::new(0),
            blocks: AtomicU64::new(0),
            local_port: AtomicU16::new(0),
            ports: Mutex::new(Vec::new()),
            in_flight: AtomicU64::new(0),
            peak_in_flight: AtomicU64::new(0),
            requests: AtomicU64::new(0),
            service_nanos: AtomicU64::new(0),
        }
    }
}

/// What one bridge's request pipeline is doing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BridgePipeline {
    /// Blocks requested and not yet served.
    pub in_flight: u64,
    /// The most that were ever outstanding at once.
    pub peak_in_flight: u64,
    /// Blocks the session asked for.
    pub requests: u64,
    /// Blocks served.
    pub blocks: u64,
    /// Total request-to-answer time across those blocks.
    pub service_nanos: u64,
}

impl BridgePipeline {
    /// Mean time to answer one block, in microseconds. `None` when nothing
    /// has been served.
    pub fn mean_service_us(&self) -> Option<u64> {
        match self.blocks {
            0 => None,
            blocks => Some(self.service_nanos / blocks / 1000),
        }
    }

    /// What happened between an earlier reading and this one.
    ///
    /// The two gauges are levels rather than counts, so they are taken from
    /// the later reading rather than subtracted.
    pub fn since(&self, earlier: &Self) -> Self {
        Self {
            in_flight: self.in_flight,
            peak_in_flight: self.peak_in_flight,
            requests: self.requests.saturating_sub(earlier.requests),
            blocks: self.blocks.saturating_sub(earlier.blocks),
            service_nanos: self.service_nanos.saturating_sub(earlier.service_nanos),
        }
    }
}

impl BridgeStatus {
    /// What the bridge is doing.
    pub fn state(&self) -> BridgeState {
        *self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The last problem reported, if any.
    pub fn error(&self) -> Option<String> {
        self.error.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Bytes handed to the session.
    pub fn served_bytes(&self) -> u64 {
        self.served_bytes.load(Ordering::Relaxed)
    }

    /// Blocks handed to the session.
    pub fn blocks(&self) -> u64 {
        self.blocks.load(Ordering::Relaxed)
    }

    /// The loopback port this bridge is connected from right now, if it is
    /// connected.
    pub fn local_port(&self) -> Option<u16> {
        match self.local_port.load(Ordering::Relaxed) {
            0 => None,
            port => Some(port),
        }
    }

    /// Every loopback port this bridge has connected from, newest last.
    ///
    /// This is what tells a bridge apart from a real peer in the peer list,
    /// and it has to be the history rather than the current port: the session
    /// keeps a dead peer's row after the connection closes, and a bridge that
    /// disconnected is still not a swarm member.
    pub fn local_ports(&self) -> Vec<u16> {
        self.ports.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn set_state(&self, state: BridgeState) {
        *self.state.lock().unwrap_or_else(|e| e.into_inner()) = state;
    }

    fn set_error(&self, reason: Option<String>) {
        *self.error.lock().unwrap_or_else(|e| e.into_inner()) = reason;
    }

    fn set_local_port(&self, port: u16) {
        self.local_port.store(port, Ordering::Relaxed);
        if port == 0 {
            return;
        }
        let mut ports = self.ports.lock().unwrap_or_else(|e| e.into_inner());
        if ports.last() == Some(&port) {
            return;
        }
        // A run that reconnects for hours would otherwise keep one port per
        // attempt. The cap is generous against the number of dead peer rows a
        // session holds and small enough to be free.
        if ports.len() >= MAX_REMEMBERED_PORTS {
            ports.remove(0);
        }
        ports.push(port);
    }

    fn add_served(&self, bytes: u64) {
        self.served_bytes.fetch_add(bytes, Ordering::Relaxed);
        self.blocks.fetch_add(1, Ordering::Relaxed);
    }

    /// Everything the request pipeline is doing right now.
    pub fn pipeline(&self) -> BridgePipeline {
        BridgePipeline {
            in_flight: self.in_flight.load(Ordering::Relaxed),
            peak_in_flight: self.peak_in_flight.load(Ordering::Relaxed),
            requests: self.requests.load(Ordering::Relaxed),
            blocks: self.blocks.load(Ordering::Relaxed),
            service_nanos: self.service_nanos.load(Ordering::Relaxed),
        }
    }

    /// The session asked for a block.
    fn request_received(&self) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        let now = self.in_flight.fetch_add(1, Ordering::Relaxed) + 1;
        self.peak_in_flight.fetch_max(now, Ordering::Relaxed);
    }

    /// A requested block is no longer outstanding, whether it was served, was
    /// cancelled, or failed.
    fn request_settled(&self, elapsed: Duration) {
        self.in_flight
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                Some(n.saturating_sub(1))
            })
            .ok();
        self.service_nanos.fetch_add(
            elapsed.as_nanos().min(u128::from(u64::MAX)) as u64,
            Ordering::Relaxed,
        );
    }

    /// Drop every outstanding request. Called when the connection ends: the
    /// session will ask again on the next one, and counting the old requests
    /// as still in flight would report a depth that no longer exists.
    fn reset_in_flight(&self) {
        self.in_flight.store(0, Ordering::Relaxed);
    }
}

/// Everything a bridge needs to present one source as a peer.
#[derive(Debug, Clone)]
pub struct BridgeParams {
    /// Where the session accepts incoming peer connections.
    pub listen_addr: SocketAddr,
    /// The torrent to attach to.
    pub info_hash: Id20,
    /// The session's own peer id, so the bridge avoids colliding with it.
    pub session_peer_id: Id20,
    /// Length of a non-final piece.
    pub piece_length: u32,
    /// Pieces in the torrent.
    pub total_pieces: u32,
    /// Piece indices this source can serve in full.
    pub pieces: Vec<u32>,
    /// Concurrent HTTP fetches.
    pub concurrency: usize,
    /// Client string sent in the extended handshake.
    pub client: String,
}

impl BridgeParams {
    /// Build the parameters for one binding against one torrent.
    ///
    /// The piece list is the binding's scope narrowed to whole pieces, which
    /// is the only set the source can serve without help.
    pub fn for_binding(
        listen_addr: SocketAddr,
        info_hash: Id20,
        session_peer_id: Id20,
        layout: &Layout,
        binding: &crate::webseed::binding::Binding,
        concurrency: usize,
    ) -> Self {
        Self {
            listen_addr,
            info_hash,
            session_peer_id,
            piece_length: layout.piece_length,
            total_pieces: layout.piece_count(),
            pieces: binding.scope.whole_pieces(layout),
            concurrency,
            client: format!("bit-cli/{}", crate::VERSION),
        }
    }

    /// Whether this source can serve the whole torrent.
    pub fn is_complete(&self) -> bool {
        self.pieces.len() as u32 == self.total_pieces
    }

    /// Size of the piece bitfield in bytes, as the wire format requires.
    pub fn bitfield_bytes(&self) -> usize {
        (self.total_pieces as usize).div_ceil(8)
    }
}

/// Why a bridge connection ended.
enum BridgeError {
    /// The source is unusable. Give up on it.
    Source(String),
    /// The connection to the session failed. Reconnect later.
    Link(String),
    /// One request failed in a way that could still recover: the mirror is
    /// down, not wrong. Reconnect and let the source's own error budget decide
    /// when it has had enough. See [`retryable_failure`].
    Stalled(String),
}

/// Run a bridge until the source fails or the task is dropped.
///
/// Link failures retry with backoff, because a torrent that is not live yet
/// looks exactly like one from here. A source failure is terminal: the bridge
/// cannot retract a bitfield it has already sent, so staying connected while
/// refusing requests would only make the session wait out request timeouts.
///
/// A request that failed transiently and ran out of its own retries is
/// neither. The mirror answered, wrongly, and might answer correctly next
/// time: a 503 during a restart, or a 403 from a signature the caller told
/// `bit-cli` to retry. Those reconnect like a link failure, and what stops the
/// loop is the source's own budget: `max_errors` consecutive failed requests
/// trip its cooldown, and a cooling source's next fetch is permanent, which
/// retires it. Without that, one four-second outage killed a mirror for the
/// rest of the run and `--web-seed-max-errors` could never be reached. See
/// `TODO/multi-source.md`, T-130.
pub async fn run(params: BridgeParams, fetcher: Arc<Fetcher>, status: Arc<BridgeStatus>) {
    if params.pieces.is_empty() {
        status.set_state(BridgeState::Idle);
        status.set_error(Some(
            "the source's scope does not cover any whole piece, so it has nothing to serve"
                .to_string(),
        ));
        return;
    }

    let mut delay = RECONNECT_BASE;
    loop {
        status.set_state(BridgeState::Connecting);
        let outcome = serve(&params, &fetcher, &status).await;
        status.set_local_port(0);
        status.reset_in_flight();
        match outcome {
            Ok(()) => delay = RECONNECT_BASE,
            Err(BridgeError::Source(reason)) => {
                status.set_error(Some(reason));
                status.set_state(BridgeState::Failed);
                return;
            }
            Err(BridgeError::Link(reason)) => status.set_error(Some(reason)),
            Err(BridgeError::Stalled(reason)) => {
                // The budget running out is decided here rather than on the
                // next fetch, so the reported reason is the run of errors and
                // not the refusal that followed it.
                status.set_error(Some(reason.clone()));
                if fetcher.stats().budget_spent() {
                    let deadline = fetcher.stats().cooldown_until();
                    match (deadline, fetcher.stats().cooldown_remaining()) {
                        // Nothing to wait for. `--web-seed-cooldown 0`, the
                        // default, means the source does not come back.
                        (_, None) => {
                            status.set_state(BridgeState::Failed);
                            return;
                        }
                        (Some(deadline), Some(remaining)) => {
                            status.set_state(BridgeState::Cooling);
                            tokio::time::sleep(remaining).await;
                            fetcher.stats().end_cooldown(deadline);
                            // The mirror has had its time. Dial straight away
                            // rather than adding the link backoff on top of a
                            // wait the caller already chose.
                            delay = RECONNECT_BASE;
                            continue;
                        }
                        (None, Some(_)) => {
                            unreachable!("cooldown_remaining is Some only when cooldown_until is")
                        }
                    }
                }
            }
        }
        status.set_state(BridgeState::Connecting);
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(RECONNECT_MAX);
    }
}

/// Connect to the session and serve requests until the connection ends.
async fn serve(
    params: &BridgeParams,
    fetcher: &Arc<Fetcher>,
    status: &Arc<BridgeStatus>,
) -> Result<(), BridgeError> {
    let mut stream = TcpStream::connect(params.listen_addr)
        .await
        .map_err(|e| BridgeError::Link(format!("connect: {e}")))?;
    let _ = stream.set_nodelay(true);
    if let Ok(addr) = stream.local_addr() {
        status.set_local_port(addr.port());
    }

    let (mut read, mut write) = stream.split();
    let mut frames = Framer::default();

    handshake(params, &mut read, &mut write, &mut frames).await?;
    send_greeting(params, &mut write).await?;
    status.set_error(None);
    status.set_state(BridgeState::Active);

    // Requests the session is still waiting on. Serving a piece it cancelled
    // makes it drop the peer, so a cancel has to be honoured.
    let pending: Arc<Mutex<HashSet<BlockKey>>> = Arc::default();
    let limiter = Arc::new(Semaphore::new(params.concurrency.max(1)));
    let (out_tx, mut out_rx) = mpsc::channel::<Vec<u8>>(64);
    let mut tasks: JoinSet<Result<(), BlockFailure>> = JoinSet::new();
    let mut keep_alive = tokio::time::interval(KEEP_ALIVE_INTERVAL);
    let served: HashSet<u32> = params.pieces.iter().copied().collect();

    loop {
        // Drain what is already buffered before waiting for more.
        while let Some(frame) = frames.take_frame().map_err(BridgeError::Link)? {
            let message = Message::deserialize(&frame, &[])
                .map_err(|e| BridgeError::Link(format!("bad message: {e:?}")))?
                .0;
            match message {
                Message::Request(request) => {
                    if request.length > MAX_REQUEST_LEN {
                        return Err(BridgeError::Link(format!(
                            "session asked for {} bytes in one block",
                            request.length
                        )));
                    }
                    // A request for a piece this source never announced is a
                    // session bug, and answering it would fetch bytes outside
                    // the scope. Refuse loudly rather than silently.
                    if !served.contains(&request.index) {
                        return Err(BridgeError::Link(format!(
                            "session asked for piece {}, which this source did not announce",
                            request.index
                        )));
                    }
                    let key = (request.index, request.begin, request.length);
                    pending
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(key);
                    status.request_received();
                    tasks.spawn(serve_block(
                        key,
                        offset_of(params, request.index, request.begin),
                        limiter.clone(),
                        fetcher.clone(),
                        status.clone(),
                        pending.clone(),
                        out_tx.clone(),
                    ));
                }
                Message::Cancel(request) => {
                    pending.lock().unwrap_or_else(|e| e.into_inner()).remove(&(
                        request.index,
                        request.begin,
                        request.length,
                    ));
                }
                // The bridge only seeds, so what the session says about its
                // own progress, interest, or extensions changes nothing here.
                _ => {}
            }
        }

        tokio::select! {
            read = frames.fill(&mut read) => {
                match read {
                    Ok(0) => return Err(BridgeError::Link("session closed the connection".into())),
                    Ok(_) => {}
                    Err(e) => return Err(BridgeError::Link(format!("read: {e}"))),
                }
            }
            Some(message) = out_rx.recv() => {
                write.write_all(&message).await
                    .map_err(|e| BridgeError::Link(format!("write: {e}")))?;
            }
            Some(finished) = tasks.join_next(), if !tasks.is_empty() => {
                match finished {
                    Ok(Ok(())) => {}
                    Ok(Err(failure)) => return Err(retryable_failure(failure)),
                    Err(e) if e.is_panic() => {
                        return Err(BridgeError::Source(format!("bridge task panicked: {e}")));
                    }
                    Err(_) => {}
                }
            }
            _ = keep_alive.tick() => {
                write.write_all(&KEEP_ALIVE).await
                    .map_err(|e| BridgeError::Link(format!("keep-alive: {e}")))?;
            }
        }
    }
}

/// Exchange handshakes and confirm the session routed us to the right torrent.
async fn handshake(
    params: &BridgeParams,
    read: &mut (impl tokio::io::AsyncRead + Unpin),
    write: &mut (impl tokio::io::AsyncWrite + Unpin),
    frames: &mut Framer,
) -> Result<(), BridgeError> {
    let mut peer_id = generate_peer_id(PEER_ID_PREFIX);
    while peer_id == params.session_peer_id {
        peer_id = generate_peer_id(PEER_ID_PREFIX);
    }

    // `Handshake::new` sets the BEP 10 extension bit, which is what carries
    // the BEP 21 `upload_only` flag in the extended handshake below.
    let ours = Handshake::new(params.info_hash, peer_id);
    let mut buf = [0u8; HANDSHAKE_LEN];
    let len = ours.serialize_unchecked_len(&mut buf);
    write
        .write_all(&buf[..len])
        .await
        .map_err(|e| BridgeError::Link(format!("write handshake: {e}")))?;

    loop {
        match Handshake::deserialize(frames.buffered()) {
            Ok((theirs, size)) => {
                if theirs.info_hash != params.info_hash {
                    return Err(BridgeError::Link(
                        "session sent a different infohash".into(),
                    ));
                }
                frames.consume(size);
                return Ok(());
            }
            Err(_) => {
                let n = frames
                    .fill(read)
                    .await
                    .map_err(|e| BridgeError::Link(format!("read handshake: {e}")))?;
                if n == 0 {
                    return Err(BridgeError::Link("session closed during handshake".into()));
                }
            }
        }
    }
}

/// Announce what this source holds, then unchoke.
///
/// The order matters: the extended handshake carries the BEP 21 flag and has
/// to arrive before the bitfield, so the session knows it is looking at a
/// partial seed rather than a peer that is still downloading.
async fn send_greeting(
    params: &BridgeParams,
    write: &mut (impl tokio::io::AsyncWrite + Unpin),
) -> Result<(), BridgeError> {
    let bits = bitfield(params);
    let mut out = extended_handshake(params);
    for message in [Message::Bitfield(ByteBuf(&bits)), Message::Unchoke] {
        out.extend_from_slice(&serialize(&message, bits.len())?);
    }
    write
        .write_all(&out)
        .await
        .map_err(|e| BridgeError::Link(format!("write bitfield: {e}")))
}

/// The piece bitfield for this source's scope.
///
/// Bit `i` is set when the source covers piece `i` in full. Spare bits past
/// the last piece stay zero, as the wire format requires.
fn bitfield(params: &BridgeParams) -> Vec<u8> {
    let mut bits = vec![0u8; params.bitfield_bytes()];
    for &piece in &params.pieces {
        if piece >= params.total_pieces {
            continue;
        }
        let index = piece as usize;
        bits[index / 8] |= 0x80 >> (index % 8);
    }
    bits
}

/// The BEP 10 extended handshake, carrying the BEP 21 partial seed flag.
///
/// The dictionary is written by hand because the bridge supports no extension
/// messages at all, and an empty `m` is the honest way to say so. Keys are in
/// ascending byte order, as bencode requires.
fn extended_handshake(params: &BridgeParams) -> Vec<u8> {
    let client = &params.client;
    let mut dict = Vec::new();
    dict.push(b'd');
    dict.extend_from_slice(b"1:mde");
    dict.extend_from_slice(format!("4:reqqi{REQUEST_QUEUE}e").as_bytes());
    // BEP 21. A source that holds only part of the payload says so, so the
    // session treats it as a partial seed rather than as a leecher.
    if !params.is_complete() {
        dict.extend_from_slice(b"11:upload_onlyi1e");
    }
    dict.extend_from_slice(format!("1:v{}:{client}", client.len()).as_bytes());
    dict.push(b'e');

    let mut out = Vec::with_capacity(dict.len() + 6);
    out.extend_from_slice(&((dict.len() as u32) + 2).to_be_bytes());
    out.push(MSGID_EXTENDED);
    out.push(EXTENDED_HANDSHAKE);
    out.extend_from_slice(&dict);
    out
}

/// Serialize one message into a fresh buffer.
///
/// `librqbit` serializes into a caller-sized slice, so `payload` is the size
/// of whatever variable-length body the message carries.
fn serialize(message: &Message<'_>, payload: usize) -> Result<Vec<u8>, BridgeError> {
    let mut buf = vec![0u8; MESSAGE_OVERHEAD + payload];
    let len = message
        .serialize(&mut buf, &Default::default)
        .map_err(|e| BridgeError::Link(format!("serialize: {e}")))?;
    buf.truncate(len);
    Ok(buf)
}

/// A block the session asked for, as `(piece, offset in piece, length)`.
type BlockKey = (u32, u32, u32);

/// Why one block could not be served, and whether the source is finished.
///
/// The fetcher has already spent this request's `retries` by the time an error
/// gets here. What it has not spent is the source's error budget, so a failure
/// that could recover is kept apart from one that cannot.
struct BlockFailure {
    reason: String,
    /// Whether the source could still answer a later request.
    recoverable: bool,
}

impl From<FetchError> for BlockFailure {
    fn from(err: FetchError) -> Self {
        let recoverable = err.is_retryable();
        let reason = match err {
            FetchError::Transient { reason, .. }
            | FetchError::Permanent { reason, .. }
            | FetchError::HashMismatch { reason } => reason,
        };
        Self {
            reason,
            recoverable,
        }
    }
}

impl BlockFailure {
    /// A failure inside the bridge that has nothing to do with the source.
    fn local(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            recoverable: false,
        }
    }
}

/// Turn a block failure into the reason a connection ended.
fn retryable_failure(failure: BlockFailure) -> BridgeError {
    match failure.recoverable {
        true => BridgeError::Stalled(failure.reason),
        false => BridgeError::Source(failure.reason),
    }
}

/// The text inside a [`BridgeError`], whichever kind it is.
fn reason_of(err: BridgeError) -> String {
    match err {
        BridgeError::Source(reason) | BridgeError::Link(reason) | BridgeError::Stalled(reason) => {
            reason
        }
    }
}

/// Fetch one block over HTTP and queue it, unless the session cancelled it.
async fn serve_block(
    key: BlockKey,
    offset: u64,
    limiter: Arc<Semaphore>,
    fetcher: Arc<Fetcher>,
    status: Arc<BridgeStatus>,
    pending: Arc<Mutex<HashSet<BlockKey>>>,
    out: mpsc::Sender<Vec<u8>>,
) -> Result<(), BlockFailure> {
    // The clock starts when the request was taken off the wire, not when this
    // task gets a permit: time spent waiting for the concurrency limit is
    // time the session was waiting, and hiding it would report a pipeline
    // that answers faster than it does.
    let started = std::time::Instant::now();
    let outcome = fetch_and_send(key, offset, limiter, fetcher, &status, pending, out).await;
    status.request_settled(started.elapsed());
    outcome
}

async fn fetch_and_send(
    key: BlockKey,
    offset: u64,
    limiter: Arc<Semaphore>,
    fetcher: Arc<Fetcher>,
    status: &BridgeStatus,
    pending: Arc<Mutex<HashSet<BlockKey>>>,
    out: mpsc::Sender<Vec<u8>>,
) -> Result<(), BlockFailure> {
    let (index, begin, length) = key;
    let _permit = limiter
        .acquire()
        .await
        .map_err(|e| BlockFailure::local(e.to_string()))?;
    let block = match fetcher.read(offset, u64::from(length)).await {
        Ok(block) => block,
        Err(err) => return Err(BlockFailure::from(err)),
    };

    if !pending
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&key)
    {
        return Ok(());
    }

    let message = Message::Piece(Piece::from_data(index, begin, &block));
    let buf = serialize(&message, block.len()).map_err(|e| BlockFailure::local(reason_of(e)))?;
    status.add_served(block.len() as u64);
    let _ = out.send(buf).await;
    Ok(())
}

/// Torrent byte offset of a block within a piece.
fn offset_of(params: &BridgeParams, piece: u32, begin: u32) -> u64 {
    u64::from(piece) * u64::from(params.piece_length) + u64::from(begin)
}

/// Length-prefixed message framing over a byte stream.
///
/// Buffered bytes live outside the read future, which keeps [`Framer::fill`]
/// cancel-safe and usable directly in a `select!`.
#[derive(Default)]
struct Framer {
    buf: Vec<u8>,
}

impl Framer {
    /// Bytes received but not yet consumed.
    fn buffered(&self) -> &[u8] {
        &self.buf
    }

    /// Drop the first `n` buffered bytes.
    fn consume(&mut self, n: usize) {
        self.buf.drain(..n);
    }

    /// Read whatever is available. Zero means end of stream.
    async fn fill(
        &mut self,
        read: &mut (impl tokio::io::AsyncRead + Unpin),
    ) -> std::io::Result<usize> {
        let mut chunk = [0u8; 8192];
        let n = read.read(&mut chunk).await?;
        self.buf.extend_from_slice(&chunk[..n]);
        Ok(n)
    }

    /// Take one complete length-prefixed frame, if the buffer holds one.
    fn take_frame(&mut self) -> Result<Option<Vec<u8>>, String> {
        let Some(prefix) = self.buf.get(..4) else {
            return Ok(None);
        };
        let len = u32::from_be_bytes(prefix.try_into().unwrap_or([0; 4])) as usize;
        if len > MAX_FRAME_LEN {
            return Err(format!("session sent a {len} byte frame"));
        }
        if self.buf.len() < 4 + len {
            return Ok(None);
        }
        Ok(Some(self.buf.drain(..4 + len).collect()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webseed::binding::{BindingSet, Origin, SourceSpec};
    use crate::webseed::scope::Scope;

    const HASH: &str = "0102030405060708090a0b0c0d0e0f1011121314";

    fn layout() -> Layout {
        // Four pieces of 1024 bytes, then a short fifth.
        Layout::from_lengths("payload", false, 1024, [("payload".to_string(), 4500u64)])
    }

    fn params(scope: &str) -> BridgeParams {
        let layout = layout();
        let spec = SourceSpec::new("https://e.example/payload", Origin::CommandLine)
            .with_scope(Scope::parse(scope).unwrap());
        let set = BindingSet::resolve(&layout, HASH, &[spec]).unwrap();
        BridgeParams::for_binding(
            "127.0.0.1:1".parse().unwrap(),
            Id20::new([0u8; 20]),
            Id20::new([1u8; 20]),
            &layout,
            &set.bindings[0],
            4,
        )
    }

    #[test]
    fn block_offsets_are_absolute() {
        let p = params("*");
        assert_eq!(offset_of(&p, 0, 0), 0);
        assert_eq!(offset_of(&p, 0, 512), 512);
        assert_eq!(offset_of(&p, 3, 16), 3 * 1024 + 16);
    }

    #[test]
    fn a_whole_torrent_source_announces_every_piece() {
        let p = params("*");
        assert_eq!(p.total_pieces, 5);
        assert_eq!(p.pieces, vec![0, 1, 2, 3, 4]);
        assert!(p.is_complete());
        // Five pieces occupy one byte, so the low three bits stay clear.
        assert_eq!(bitfield(&p), vec![0b1111_1000]);
    }

    #[test]
    fn a_scoped_source_announces_only_the_pieces_it_covers_in_full() {
        let p = params("byte:0-2048");
        assert!(!p.is_complete());
        assert_eq!(
            p.pieces,
            vec![0, 1],
            "bytes 0-2047 are exactly pieces 0 and 1"
        );
        assert_eq!(bitfield(&p), vec![0b1100_0000]);
    }

    #[test]
    fn a_partially_covered_piece_is_never_announced() {
        // Bytes 0 to 1535 cover piece 0 in full and only half of piece 1.
        // Announcing piece 1 would make the session request bytes this source
        // cannot serve, and the piece would never verify.
        let p = params("byte:0-1536");
        assert_eq!(p.pieces, vec![0]);
        assert_eq!(bitfield(&p), vec![0b1000_0000]);
    }

    #[test]
    fn the_bitfield_is_the_right_length_for_the_piece_count() {
        let layout = Layout::from_lengths("t", false, 1024, [("t".to_string(), 12 * 1024u64)]);
        let spec = SourceSpec::new("https://e.example/t", Origin::CommandLine);
        let set = BindingSet::resolve(&layout, HASH, &[spec]).unwrap();
        let p = BridgeParams::for_binding(
            "127.0.0.1:1".parse().unwrap(),
            Id20::new([0u8; 20]),
            Id20::new([1u8; 20]),
            &layout,
            &set.bindings[0],
            1,
        );
        assert_eq!(p.total_pieces, 12);
        assert_eq!(p.bitfield_bytes(), 2);
        // Twelve pieces in two bytes, so the low four bits are spare.
        assert_eq!(bitfield(&p), vec![0xFF, 0xF0]);
    }

    #[test]
    fn a_complete_source_does_not_claim_to_be_upload_only() {
        let dict = extended_handshake(&params("*"));
        let text = String::from_utf8_lossy(&dict).into_owned();
        assert!(!text.contains("upload_only"), "{text}");
    }

    #[test]
    fn a_partial_source_advertises_bep_21() {
        let dict = extended_handshake(&params("byte:0-2048"));
        let text = String::from_utf8_lossy(&dict).into_owned();
        assert!(text.contains("11:upload_onlyi1e"), "{text}");
    }

    #[test]
    fn the_extended_handshake_is_a_well_formed_frame() {
        let frame = extended_handshake(&params("*"));
        let len = u32::from_be_bytes(frame[..4].try_into().unwrap()) as usize;
        assert_eq!(
            len,
            frame.len() - 4,
            "the length prefix covers the rest of the frame"
        );
        assert_eq!(frame[4], MSGID_EXTENDED);
        assert_eq!(frame[5], EXTENDED_HANDSHAKE);
        assert_eq!(frame[6], b'd', "the payload is a bencoded dictionary");
        assert_eq!(frame[frame.len() - 1], b'e');
    }

    #[test]
    fn the_extended_handshake_advertises_no_extension_messages() {
        // An empty `m` is what says "I speak the extension protocol but
        // implement none of its messages", which is exactly true here.
        let frame = extended_handshake(&params("*"));
        let text = String::from_utf8_lossy(&frame).into_owned();
        assert!(text.contains("1:mde"), "{text}");
        assert!(!text.contains("ut_metadata"), "{text}");
        assert!(!text.contains("ut_pex"), "{text}");
    }

    #[test]
    fn the_extended_handshake_keys_are_in_bencode_order() {
        let frame = extended_handshake(&params("byte:0-2048"));
        let text = String::from_utf8_lossy(&frame).into_owned();
        let m = text.find("1:m").unwrap();
        let reqq = text.find("4:reqq").unwrap();
        let upload = text.find("11:upload_only").unwrap();
        let v = text.rfind("1:v").unwrap();
        assert!(m < reqq && reqq < upload && upload < v, "{text}");
    }

    #[test]
    fn framer_yields_whole_frames_only() {
        let mut framer = Framer::default();
        framer.buf.extend_from_slice(&[0, 0, 0, 2, 9]);
        assert_eq!(framer.take_frame().unwrap(), None);
        framer.buf.push(7);
        assert_eq!(framer.take_frame().unwrap(), Some(vec![0, 0, 0, 2, 9, 7]));
        assert_eq!(framer.take_frame().unwrap(), None);
    }

    #[test]
    fn framer_handles_keep_alives_and_back_to_back_frames() {
        let mut framer = Framer::default();
        framer
            .buf
            .extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0]);
        assert_eq!(framer.take_frame().unwrap(), Some(vec![0, 0, 0, 0]));
        assert_eq!(framer.take_frame().unwrap(), Some(vec![0, 0, 0, 1, 1]));
        assert_eq!(framer.take_frame().unwrap(), Some(vec![0, 0, 0, 0]));
        assert_eq!(framer.take_frame().unwrap(), None);
    }

    #[test]
    fn framer_rejects_absurd_frames() {
        let mut framer = Framer::default();
        framer.buf.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
        assert!(framer.take_frame().is_err());
    }

    #[test]
    fn a_fresh_status_reports_nothing_served() {
        let status = BridgeStatus::default();
        assert_eq!(status.state(), BridgeState::Connecting);
        assert_eq!(status.served_bytes(), 0);
        assert_eq!(status.blocks(), 0);
        assert_eq!(status.local_port(), None);
        assert_eq!(status.error(), None);

        status.add_served(1024);
        status.add_served(512);
        assert_eq!(status.served_bytes(), 1536);
        assert_eq!(status.blocks(), 2);
    }
}
