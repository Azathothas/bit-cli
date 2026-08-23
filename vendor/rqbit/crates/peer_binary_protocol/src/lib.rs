// BitTorrent peer protocol implementation: parsing, serialization etc.
//
// Can be used outside of librqbit.

mod double_buf;
pub mod extended;

use std::hint::unreachable_unchecked;

use buffers::{ByteBuf, ByteBufOwned};
use byteorder::{BE, ByteOrder};
use bytes::Bytes;
use clone_to_owned::CloneToOwned;
use extended::PeerExtendedMessageIds;
use librqbit_core::{constants::CHUNK_SIZE, hash_id::Id20, lengths::ChunkInfo};
use serde_derive::{Deserialize, Serialize};

pub use crate::double_buf::DoubleBufHelper;

use self::extended::ExtendedMessage;

const INTEGER_LEN: usize = 4;
const MSGID_LEN: usize = 1;
const PREAMBLE_LEN: usize = INTEGER_LEN + MSGID_LEN;
const PIECE_MESSAGE_PREAMBLE_LEN: usize = PREAMBLE_LEN + INTEGER_LEN * 2;
pub const PIECE_MESSAGE_DEFAULT_LEN: usize = PIECE_MESSAGE_PREAMBLE_LEN + CHUNK_SIZE as usize;

// The ut_metadata data message is the largest message whose length the
// protocol bounds. The bitfield is NOT bounded by this: it carries one bit per
// piece, so its length is a property of the torrent. Size a bitfield buffer
// with Message::bitfield_message_len instead.
const MAX_MSG_LEN_LEN_JUST_IN_CASE_EXTRA: usize = 64;
pub const MAX_MSG_LEN: usize = PREAMBLE_LEN
    + 1
    + b"d8:msg_typei1e5:piecei42e10:total_sizei16384ee".len()
    + CHUNK_SIZE as usize
    + MAX_MSG_LEN_LEN_JUST_IN_CASE_EXTRA;

const PSTR_BT1: &str = "BitTorrent protocol";

type MsgId = u8;

const MSGID_CHOKE: MsgId = 0;
const MSGID_UNCHOKE: MsgId = 1;
const MSGID_INTERESTED: MsgId = 2;
const MSGID_NOT_INTERESTED: MsgId = 3;
const MSGID_HAVE: MsgId = 4;
const MSGID_BITFIELD: MsgId = 5;
const MSGID_REQUEST: MsgId = 6;
const MSGID_PIECE: MsgId = 7;
const MSGID_CANCEL: MsgId = 8;
// BEP 6, the fast extension. Five ids in a block, and this crate declared none
// of them: a peer that spoke BEP 6 was answered with `UnsupportedMessageId`
// and dropped. See TODO/bep-coverage.md T-100.
const MSGID_SUGGEST_PIECE: MsgId = 13;
const MSGID_HAVE_ALL: MsgId = 14;
const MSGID_HAVE_NONE: MsgId = 15;
const MSGID_REJECT_REQUEST: MsgId = 16;
const MSGID_ALLOWED_FAST: MsgId = 17;
const MSGID_EXTENDED: MsgId = 20;

/// BEP 6's reserved bit: the last reserved byte, `0x04`.
///
/// BEP 10's is byte 5 and `0x10`, which is `1 << 20` in the `u64` this crate
/// keeps the reserved bytes in. This one is `1 << 2`.
const RESERVED_FAST: u64 = 1 << 2;

pub const EXTENDED_UT_METADATA_KEY: &[u8] = b"ut_metadata";
pub const MY_EXTENDED_UT_METADATA: u8 = 3;

pub const EXTENDED_UT_PEX_KEY: &[u8] = b"ut_pex";
pub const MY_EXTENDED_UT_PEX: u8 = 1;

/// BEP 54 `lt_donthave`, the inverse of `Have`.
///
/// A peer's bitfield only ever grew: BEP 3 has `Have` and nothing that
/// withdraws a claim, so a peer that loses a file has no way to say so and the
/// far end keeps asking it for pieces it cannot serve. This is the number this
/// client advertises for that message in its own extended handshake, so it is
/// the number a peer will use when it sends us one. See TODO/bep-coverage.md
/// T-167.
pub const MY_EXTENDED_LT_DONTHAVE: u8 = 4;

#[derive(Clone, Copy)]
pub struct MsgIdDebug(MsgId);
impl MsgIdDebug {
    const fn name(&self) -> Option<&'static str> {
        let n = match self.0 {
            MSGID_CHOKE => "choke",
            MSGID_UNCHOKE => "unchoke",
            MSGID_INTERESTED => "interested",
            MSGID_NOT_INTERESTED => "not_interested",
            MSGID_HAVE => "have",
            MSGID_BITFIELD => "bitfield",
            MSGID_REQUEST => "request",
            MSGID_PIECE => "piece",
            MSGID_CANCEL => "cancel",
            MSGID_SUGGEST_PIECE => "suggest_piece",
            MSGID_HAVE_ALL => "have_all",
            MSGID_HAVE_NONE => "have_none",
            MSGID_REJECT_REQUEST => "reject_request",
            MSGID_ALLOWED_FAST => "allowed_fast",
            MSGID_EXTENDED => "extended",
            _ => return None,
        };
        Some(n)
    }
}
impl core::fmt::Debug for MsgIdDebug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.name() {
            Some(name) => f.write_str(name),
            None => write!(f, "<unknown msg_id {}>", self.0),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum MessageDeserializeError {
    #[error("not enough data (msgid={1:?}): expected at least {0} more bytes")]
    NotEnoughData(usize, Option<MsgIdDebug>),
    #[error("need a contiguous input to deserialize")]
    NeedContiguous,
    #[error("unsupported message id {0}")]
    UnsupportedMessageId(u8),
    #[error(transparent)]
    Bencode(#[from] bencode::DeserializeError),
    #[error("incorrect message length msg_id={msg_id:?}, expected={expected}, received={received}")]
    IncorrectMsgLen {
        received: u32,
        expected: u32,
        msg_id: MsgIdDebug,
    },
    #[error("ut_metadata:data received {received_len} >= total_size is {total_size}")]
    UtMetadataBufLargerThanTotalSize { total_size: u32, received_len: u32 },
    #[error("ut_metadata:data length must be <= {CHUNK_SIZE} but received {0} bytes")]
    UtMetadataTooLarge(u32),
    #[error("ut_metadata: trailing bytes when decoding")]
    UtMetadataTrailingBytes,
    #[error("ut_metadata: missing total_size")]
    UtMetadataMissingTotalSize,
    #[error("ut_metadata: unrecognized message type: {0}")]
    UtMetadataTypeUnknown(u32),
    #[error("ut_metadata: received piece {received_piece} > total pieces {total_pieces}")]
    UtMetadataPieceOutOfBounds {
        total_pieces: u32,
        received_piece: u32,
    },
    #[error("ut_metadata: expected size {expected_size} != received size {received_size}")]
    UtMetadataSizeMismatch {
        expected_size: u32,
        received_size: u32,
    },
    #[error("pstr doesn't match {PSTR_BT1:?}")]
    HandshakePstrWrongContent,
    #[error("pstr should be 19 bytes long but got {0}")]
    HandshakePstrWrongLength(u8),
}

pub fn serialize_piece_preamble(chunk: &ChunkInfo, mut buf: &mut [u8]) -> usize {
    let len_prefix = MSGID_LEN as u32 + INTEGER_LEN as u32 * 2 + chunk.size;
    BE::write_u32(&mut buf[0..4], len_prefix);
    buf[4] = MSGID_PIECE;

    buf = &mut buf[5..];
    BE::write_u32(&mut buf[0..4], chunk.piece_index.get());
    BE::write_u32(&mut buf[4..8], chunk.offset);

    PIECE_MESSAGE_PREAMBLE_LEN
}

pub struct Piece<B> {
    pub index: u32,
    pub begin: u32,
    block_0: B,
    block_1: B,
}

impl<B: AsRef<[u8]>> std::fmt::Debug for Piece<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Piece")
            .field("index", &self.index)
            .field("begin", &self.begin)
            .field("len", &self.len())
            .field("len_0", &self.block_0.as_ref().len())
            .field("len_1", &self.block_1.as_ref().len())
            .finish_non_exhaustive()
    }
}

impl CloneToOwned for Piece<ByteBuf<'_>> {
    type Target = Piece<ByteBufOwned>;

    fn clone_to_owned(&self, within_buffer: Option<&Bytes>) -> Self::Target {
        Piece {
            index: self.index,
            begin: self.begin,
            block_0: self.block_0.clone_to_owned(within_buffer),
            block_1: self.block_1.clone_to_owned(within_buffer),
        }
    }
}

impl<B: AsRef<[u8]>> Piece<B> {
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.block_0.as_ref().len() + self.block_1.as_ref().len()
    }

    pub fn serialize_unchecked_len(&self, mut buf: &mut [u8]) -> usize {
        buf[0..4].copy_from_slice(&self.index.to_be_bytes());
        buf[4..8].copy_from_slice(&self.begin.to_be_bytes());
        buf = &mut buf[8..];

        let b0 = self.block_0.as_ref();
        let b1 = self.block_1.as_ref();

        buf[..b0.len()].copy_from_slice(b0);
        buf = &mut buf[b0.len()..];
        buf[..b1.len()].copy_from_slice(b1);
        8 + b0.len() + b1.len()
    }
}

impl Piece<ByteBufOwned> {
    pub fn as_borrowed(&self) -> Piece<ByteBuf<'_>> {
        Piece {
            index: self.index,
            begin: self.begin,
            block_0: self.block_0.as_ref().into(),
            block_1: self.block_1.as_ref().into(),
        }
    }
}

impl<'a> Piece<ByteBuf<'a>> {
    pub fn data(&self) -> (&'a [u8], &'a [u8]) {
        (self.block_0.0, self.block_1.0)
    }

    pub fn from_data(index: u32, begin: u32, block: &'a [u8]) -> Self {
        Piece {
            index,
            begin,
            block_0: ByteBuf(block),
            block_1: ByteBuf(&[]),
        }
    }
}

#[derive(Debug)]
pub enum Message<'a> {
    Request(Request),
    Cancel(Request),
    Bitfield(ByteBuf<'a>),
    KeepAlive,
    Have(u32),
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Piece(Piece<ByteBuf<'a>>),
    /// BEP 6. The sender has every piece, in two bytes rather than a bitfield.
    HaveAll,
    /// BEP 6. The sender has no piece.
    HaveNone,
    /// BEP 6. A piece the sender would rather you asked it for.
    SuggestPiece(u32),
    /// BEP 6. A request the sender will not answer, named so the asker can
    /// stop waiting rather than time out.
    RejectRequest(Request),
    /// BEP 6. A piece the sender will serve even while choking.
    AllowedFast(u32),
    Extended(ExtendedMessage<ByteBuf<'a>>),
}

#[derive(thiserror::Error, Debug)]
pub enum SerializeError {
    #[error("not enough space in buffer")]
    NoSpaceInBuffer,
    #[error(transparent)]
    Bencode(#[from] bencode::SerializeError),
    #[error("need peer's handshake to serialize ut_metadata, or peer does't support ut_metadata")]
    NeedUtMetadata,
    #[error("need peer's handshake to serialize ut_pex, or peer does't support ut_pex")]
    NeedPex,
    #[error("need peer's handshake to serialize lt_donthave, or peer doesn't support lt_donthave")]
    NeedLtDontHave,
}

impl From<std::io::Error> for SerializeError {
    fn from(_: std::io::Error) -> Self {
        Self::NoSpaceInBuffer
    }
}

impl Message<'_> {
    /// The exact number of bytes [`Message::serialize`] needs to write a
    /// bitfield message carrying `bitfield_len` bytes of bitfield.
    ///
    /// Every other message fits in [`MAX_MSG_LEN`]. This one does not: a
    /// bitfield is one bit per piece, so a torrent with more than
    /// `(MAX_MSG_LEN - PREAMBLE_LEN) * 8` pieces produces a message that a
    /// fixed buffer cannot hold. The caller sizes its buffer with this.
    pub const fn bitfield_message_len(bitfield_len: usize) -> usize {
        PREAMBLE_LEN + bitfield_len
    }

    pub fn serialize(
        &self,
        out: &mut [u8],
        peer_extended_messages: &dyn Fn() -> PeerExtendedMessageIds,
    ) -> Result<usize, SerializeError> {
        macro_rules! check_len {
            ($l:expr) => {
                if out.len() < $l {
                    return Err(SerializeError::NoSpaceInBuffer);
                }
            };
        }

        macro_rules! write_preamble {
            ($msg_len:expr, $msg_id:expr) => {
                out[0..4].copy_from_slice(&(($msg_len + 1u32).to_be_bytes()));
                out[4] = $msg_id;
            };
        }

        match self {
            Message::Request(request) | Message::Cancel(request) => {
                const TOTAL_LEN: usize = PREAMBLE_LEN + INTEGER_LEN * 3;
                check_len!(TOTAL_LEN);
                let msg_id = match self {
                    Message::Request(..) => MSGID_REQUEST,
                    Message::Cancel(..) => MSGID_CANCEL,
                    _ => unsafe { unreachable_unchecked() },
                };
                write_preamble!((INTEGER_LEN * 3) as u32, msg_id);
                request.serialize_unchecked_len(&mut out[PREAMBLE_LEN..]);
                Ok(TOTAL_LEN)
            }
            Message::Bitfield(b) => {
                let block_len = b.as_ref().len();
                let total_len: usize = PREAMBLE_LEN + block_len;
                check_len!(total_len);
                write_preamble!(block_len as u32, MSGID_BITFIELD);
                out[PREAMBLE_LEN..PREAMBLE_LEN + block_len].copy_from_slice(b.as_ref());
                Ok(total_len)
            }
            Message::Choke | Message::Unchoke | Message::Interested | Message::NotInterested => {
                check_len!(PREAMBLE_LEN);
                let msg_id = match self {
                    Message::Choke => MSGID_CHOKE,
                    Message::Unchoke => MSGID_UNCHOKE,
                    Message::Interested => MSGID_INTERESTED,
                    Message::NotInterested => MSGID_NOT_INTERESTED,
                    _ => unsafe { unreachable_unchecked() },
                };
                write_preamble!(0, msg_id);
                Ok(PREAMBLE_LEN)
            }
            Message::Piece(p) => {
                let block_len = p.len();
                let payload_len = INTEGER_LEN * 2 + block_len;
                let total_len = PREAMBLE_LEN + payload_len;
                check_len!(total_len);
                write_preamble!(payload_len as u32, MSGID_PIECE);
                p.serialize_unchecked_len(&mut out[PREAMBLE_LEN..]);
                Ok(total_len)
            }
            Message::KeepAlive => {
                check_len!(4);
                out[0..4].copy_from_slice(&0u32.to_be_bytes());
                Ok(4)
            }
            Message::Have(v) => {
                check_len!(PREAMBLE_LEN + INTEGER_LEN);
                write_preamble!(INTEGER_LEN as u32, MSGID_HAVE);
                out[5..9].copy_from_slice(&v.to_be_bytes());
                Ok(9)
            }
            Message::HaveAll | Message::HaveNone => {
                check_len!(PREAMBLE_LEN);
                let msg_id = match self {
                    Message::HaveAll => MSGID_HAVE_ALL,
                    Message::HaveNone => MSGID_HAVE_NONE,
                    _ => unsafe { unreachable_unchecked() },
                };
                write_preamble!(0, msg_id);
                Ok(PREAMBLE_LEN)
            }
            Message::SuggestPiece(v) | Message::AllowedFast(v) => {
                check_len!(PREAMBLE_LEN + INTEGER_LEN);
                let msg_id = match self {
                    Message::SuggestPiece(..) => MSGID_SUGGEST_PIECE,
                    Message::AllowedFast(..) => MSGID_ALLOWED_FAST,
                    _ => unsafe { unreachable_unchecked() },
                };
                write_preamble!(INTEGER_LEN as u32, msg_id);
                out[5..9].copy_from_slice(&v.to_be_bytes());
                Ok(PREAMBLE_LEN + INTEGER_LEN)
            }
            Message::RejectRequest(request) => {
                const TOTAL_LEN: usize = PREAMBLE_LEN + INTEGER_LEN * 3;
                check_len!(TOTAL_LEN);
                write_preamble!((INTEGER_LEN * 3) as u32, MSGID_REJECT_REQUEST);
                request.serialize_unchecked_len(&mut out[PREAMBLE_LEN..]);
                Ok(TOTAL_LEN)
            }
            Message::Extended(e) => {
                check_len!(PREAMBLE_LEN + 2);
                let msg_len = e.serialize(&mut out[PREAMBLE_LEN..], peer_extended_messages)?;
                write_preamble!(msg_len as u32, MSGID_EXTENDED);
                Ok(PREAMBLE_LEN + msg_len)
            }
        }
    }
}

impl Message<'_> {
    pub fn deserialize<'a>(
        buf: &'a [u8],
        buf2: &'a [u8],
    ) -> Result<(Message<'a>, usize), MessageDeserializeError> {
        let mut buf = DoubleBufHelper::new(buf, buf2);
        let len_prefix = buf
            .read_u32_be()
            .map_err(|rem| MessageDeserializeError::NotEnoughData(rem, None))?;
        let total_len = len_prefix as usize + 4;
        if len_prefix == 0 {
            return Ok((Message::KeepAlive, total_len));
        }

        let msg_id = buf.read_u8().ok_or(MessageDeserializeError::NotEnoughData(
            len_prefix as usize,
            None,
        ))?;

        let msg_len = len_prefix as usize - 1;
        if buf.len() < msg_len {
            return Err(MessageDeserializeError::NotEnoughData(
                msg_len - buf.len(),
                Some(MsgIdDebug(msg_id)),
            ));
        }

        macro_rules! check_msg_len {
            ($expected:expr) => {{
                if msg_len != $expected {
                    return Err(MessageDeserializeError::IncorrectMsgLen {
                        received: len_prefix - 1,
                        expected: $expected,
                        msg_id: MsgIdDebug(msg_id),
                    });
                }
            }};
            (min $expected:expr) => {{
                if msg_len < $expected {
                    return Err(MessageDeserializeError::IncorrectMsgLen {
                        received: len_prefix - 1,
                        expected: $expected,
                        msg_id: MsgIdDebug(msg_id),
                    });
                }
            }};
        }

        match msg_id {
            MSGID_CHOKE => {
                check_msg_len!(0);
                Ok((Message::Choke, total_len))
            }
            MSGID_UNCHOKE => {
                check_msg_len!(0);
                Ok((Message::Unchoke, total_len))
            }
            MSGID_INTERESTED => {
                check_msg_len!(0);
                Ok((Message::Interested, total_len))
            }
            MSGID_NOT_INTERESTED => {
                check_msg_len!(0);
                Ok((Message::NotInterested, total_len))
            }
            MSGID_HAVE => {
                check_msg_len!(4);
                let have = buf.read_u32_be().unwrap();
                Ok((Message::Have(have), total_len))
            }
            MSGID_BITFIELD => {
                check_msg_len!(min 1);
                // In practice, as bitfield is always (almost) the first message, it should be contiguous.
                let data = buf
                    .get_contiguous(msg_len)
                    .ok_or(MessageDeserializeError::NeedContiguous)?;
                Ok((Message::Bitfield(ByteBuf::from(data)), total_len))
            }
            MSGID_REQUEST | MSGID_CANCEL | MSGID_REJECT_REQUEST => {
                check_msg_len!(12);
                const I32: usize = 4;
                const I32_3: usize = I32 * 3;
                let req = buf.consume::<I32_3>().unwrap();
                let request = Request {
                    index: BE::read_u32(&req[0..I32]),
                    begin: BE::read_u32(&req[I32..I32 * 2]),
                    length: BE::read_u32(&req[I32 * 2..I32 * 3]),
                };
                let req = match msg_id {
                    MSGID_REQUEST => Message::Request(request),
                    MSGID_CANCEL => Message::Cancel(request),
                    _ => Message::RejectRequest(request),
                };
                Ok((req, total_len))
            }
            MSGID_HAVE_ALL => {
                check_msg_len!(0);
                Ok((Message::HaveAll, total_len))
            }
            MSGID_HAVE_NONE => {
                check_msg_len!(0);
                Ok((Message::HaveNone, total_len))
            }
            MSGID_SUGGEST_PIECE | MSGID_ALLOWED_FAST => {
                check_msg_len!(4);
                let piece = buf.read_u32_be().unwrap();
                let msg = match msg_id {
                    MSGID_SUGGEST_PIECE => Message::SuggestPiece(piece),
                    _ => Message::AllowedFast(piece),
                };
                Ok((msg, total_len))
            }
            MSGID_PIECE => {
                const MIN_PAYLOAD: usize = 1;
                const MIN_LENGTH: usize = INTEGER_LEN * 2 + MIN_PAYLOAD;
                if msg_len < MIN_LENGTH {
                    return Err(MessageDeserializeError::IncorrectMsgLen {
                        expected: MIN_LENGTH as u32,
                        received: msg_len as u32,
                        msg_id: MsgIdDebug(msg_id),
                    });
                }

                let index = buf.read_u32_be().unwrap();
                let begin = buf.read_u32_be().unwrap();

                let block_len = msg_len - INTEGER_LEN * 2;
                let (block_0, block_1) = buf.consume_variable(block_len).unwrap();

                Ok((
                    Message::Piece(Piece {
                        index,
                        begin,
                        block_0: block_0.into(),
                        block_1: block_1.into(),
                    }),
                    total_len,
                ))
            }
            MSGID_EXTENDED => Ok((
                Message::Extended(ExtendedMessage::deserialize(buf.with_max_len(msg_len))?),
                PREAMBLE_LEN + msg_len,
            )),
            msg_id => Err(MessageDeserializeError::UnsupportedMessageId(msg_id)),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Handshake {
    pub reserved: u64,
    pub info_hash: Id20,
    pub peer_id: Id20,
}

impl Handshake {
    pub fn new(info_hash: Id20, peer_id: Id20) -> Handshake {
        debug_assert_eq!(PSTR_BT1.len(), 19);

        let mut reserved: u64 = 0;
        // supports extended messaging
        reserved |= 1 << 20;
        // BEP 6. Advertised because this client understands all five messages
        // now: a peer that sends `have all` in place of a bitfield used to
        // leave us with an empty bitfield and nothing to ask it for, and a
        // `reject request` used to end the connection. See
        // TODO/bep-coverage.md T-100.
        reserved |= RESERVED_FAST;

        Handshake {
            reserved,
            info_hash,
            peer_id,
        }
    }

    pub fn deserialize(b: &[u8]) -> Result<(Handshake, usize), MessageDeserializeError> {
        const LEN: usize = 1 + PSTR_BT1.len() + 8 + 20 + 20;
        if b.len() < LEN {
            return Err(MessageDeserializeError::NotEnoughData(LEN - b.len(), None));
        }
        if b[0] as usize != PSTR_BT1.len() {
            return Err(MessageDeserializeError::HandshakePstrWrongLength(b[0]));
        }
        if &b[1..20] != PSTR_BT1.as_bytes() {
            return Err(MessageDeserializeError::HandshakePstrWrongContent);
        }

        let h = Handshake {
            reserved: BE::read_u64(&b[20..28]),
            info_hash: Id20::new(b[28..48].try_into().unwrap()),
            peer_id: Id20::new(b[48..68].try_into().unwrap()),
        };
        Ok((h, LEN))
    }

    pub fn supports_extended(&self) -> bool {
        self.reserved.to_be_bytes()[5] & 0x10 > 0
    }

    /// BEP 6, the fast extension: the last reserved byte, `0x04`.
    pub fn supports_fast(&self) -> bool {
        self.reserved.to_be_bytes()[7] & 0x04 > 0
    }

    #[must_use]
    pub fn serialize_unchecked_len(&self, buf: &mut [u8]) -> usize {
        debug_assert_eq!(PSTR_BT1.len(), 19);
        buf[0] = 19;
        buf[1..20].copy_from_slice(PSTR_BT1.as_bytes());
        buf[20..28].copy_from_slice(&self.reserved.to_be_bytes());
        buf[28..48].copy_from_slice(&self.info_hash.0);
        buf[48..68].copy_from_slice(&self.peer_id.0);
        68
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Request {
    pub index: u32,
    pub begin: u32,
    pub length: u32,
}

impl Request {
    pub fn new(index: u32, begin: u32, length: u32) -> Self {
        Self {
            index,
            begin,
            length,
        }
    }

    pub fn serialize_unchecked_len(&self, buf: &mut [u8]) -> usize {
        buf[0..4].copy_from_slice(&self.index.to_be_bytes());
        buf[4..8].copy_from_slice(&self.begin.to_be_bytes());
        buf[8..12].copy_from_slice(&self.length.to_be_bytes());
        12
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Context;

    use crate::extended::handshake::ExtendedHandshake;

    const EXTENDED: &[u8] = include_bytes!("../../librqbit/resources/test/extended-handshake.bin");

    use super::*;
    #[test]
    fn test_handshake_serialize() {
        let info_hash = Id20::new([
            1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ]);
        let peer_id = Id20::new([
            1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ]);
        let mut buf = [0u8; 100];
        let se = Handshake::new(info_hash, peer_id);
        let len = se.serialize_unchecked_len(&mut buf);
        assert_eq!(len, 20 + 20 + 8 + 19 + 1);
        assert_eq!(buf[0], 19);
        assert_eq!(&buf[1..20], PSTR_BT1.as_bytes());
        assert_eq!(&buf[28..48], &info_hash.0);
        assert_eq!(&buf[48..68], &peer_id.0);

        let (de, dlen) = Handshake::deserialize(&buf).unwrap();
        assert_eq!(dlen, len);
        assert_eq!(se, de);
    }

    #[test]
    fn test_extended_serialize() {
        let msg = Message::Extended(ExtendedMessage::Handshake(ExtendedHandshake::new()));
        let mut out = [0u8; 100];
        msg.serialize(&mut out, &Default::default).unwrap();
        dbg!(out);
    }

    #[test]
    fn test_deserialize_serialize_extended_non_contiguous() {
        for split_point in 0..EXTENDED.len() {
            let (first, second) = EXTENDED.split_at(split_point);
            let res = Message::deserialize(first, second);
            if split_point > PREAMBLE_LEN + 1 && split_point < EXTENDED.len() {
                assert!(
                    matches!(res, Err(MessageDeserializeError::NeedContiguous)),
                    "expected NeedContiguous: {split_point}"
                )
            } else {
                let (msg, len) = res
                    .inspect_err(|e| panic!("split_point={split_point:?}; error: {e:#}"))
                    .unwrap();
                assert!(matches!(msg, Message::Extended(..)));
                assert_eq!(len, EXTENDED.len());
            }
        }
    }

    #[test]
    fn test_deserialize_piece() {
        const LEN: usize = 100;
        const EXTRA: usize = 100;
        let mut buf = [0u8; LEN + EXTRA];

        #[allow(clippy::needless_range_loop)]
        for id in 0..buf.len() {
            buf[id] = id as u8;
        }

        let block_len = LEN - PREAMBLE_LEN - INTEGER_LEN * 2;
        let len_prefix: u32 = (block_len + INTEGER_LEN * 2 + MSGID_LEN) as u32;
        let index: u32 = 42;
        let begin: u32 = 43;

        buf[0..4].copy_from_slice(&len_prefix.to_be_bytes());
        buf[4] = MSGID_PIECE;
        buf[5..9].copy_from_slice(&index.to_be_bytes());
        buf[9..13].copy_from_slice(&begin.to_be_bytes());

        for split_point in 0..buf.len() {
            dbg!(split_point);
            let (first, second) = buf.split_at(split_point);
            let (msg, len) = Message::deserialize(first, second).unwrap();

            let piece = match &msg {
                Message::Piece(piece) => piece,
                other => panic!("expected piece got {other:?}"),
            };

            assert_eq!(piece.len(), block_len);
            assert_eq!(piece.index, index);
            assert_eq!(piece.begin, begin);
            assert_eq!(len, LEN);

            let mut tmp = [0u8; 100];
            let slen = msg.serialize(&mut tmp, &|| Default::default()).unwrap();
            assert_eq!(slen, len);
            assert_eq!(buf[..len], tmp[..len]);

            let (first, second) = piece.data();

            assert_eq!(first.len() + second.len(), block_len);
            assert_eq!(first, &buf[13..13 + first.len()]);
            assert_eq!(
                second,
                &buf[13 + first.len()..13 + first.len() + second.len()]
            );
        }
    }

    #[test]
    fn test_deserialize_request() {
        let mut buf = [0u8; 100];

        let len_prefix: u32 = (MSGID_LEN + INTEGER_LEN * 3) as u32;
        let index: u32 = 42;
        let begin: u32 = 43;
        let length: u32 = 44;

        buf[0..4].copy_from_slice(&len_prefix.to_be_bytes());
        buf[4] = MSGID_REQUEST;
        buf[5..9].copy_from_slice(&index.to_be_bytes());
        buf[9..13].copy_from_slice(&begin.to_be_bytes());
        buf[13..17].copy_from_slice(&length.to_be_bytes());

        for split_point in 0..buf.len() {
            dbg!(split_point);
            let (first, second) = buf.split_at(split_point);
            let (msg, len) = Message::deserialize(first, second).unwrap();

            let request = match msg {
                Message::Request(req) => req,
                other => panic!("expected request got {other:?}"),
            };

            assert_eq!(request.index, index);
            assert_eq!(request.begin, begin);
            assert_eq!(request.length, length);
            assert_eq!(len, 17);

            let mut tmp = [0u8; 100];
            let slen = msg.serialize(&mut tmp, &|| Default::default()).unwrap();
            assert_eq!(slen, len);
            assert_eq!(buf[..len], tmp[..len]);
        }
    }

    #[test]
    fn test_keepalive() {
        let buf = [0u8; 100];

        for split_point in 0..buf.len() {
            let (first, second) = buf.split_at(split_point);
            let (msg, len) = Message::deserialize(first, second).unwrap();
            assert!(matches!(msg, Message::KeepAlive));
            assert_eq!(len, 4);
            let mut tmp = [0u8; 100];
            let slen = msg.serialize(&mut tmp, &|| Default::default()).unwrap();
            assert_eq!(slen, len);
            assert_eq!(buf[..len], tmp[..len]);
        }
    }

    #[test]
    fn test_have() {
        let mut buf = [0u8; 100];
        buf[0..4].copy_from_slice(&5u32.to_be_bytes());
        buf[4] = MSGID_HAVE;
        buf[5..9].copy_from_slice(&42u32.to_be_bytes());

        for split_point in 0..buf.len() {
            let (first, second) = buf.split_at(split_point);
            let (msg, len) = Message::deserialize(first, second).unwrap();
            assert!(matches!(msg, Message::Have(42)));
            assert_eq!(len, 9);
            let mut tmp = [0u8; 100];
            let slen = msg.serialize(&mut tmp, &|| Default::default()).unwrap();
            assert_eq!(slen, len);
            assert_eq!(buf[..len], tmp[..len]);
        }
    }

    #[test]
    fn test_bitfield() {
        let mut buf = [0u8; 100];
        buf[0..4].copy_from_slice(&43u32.to_be_bytes());
        buf[4] = MSGID_BITFIELD;
        for byte in buf[5..47].iter_mut() {
            *byte = 0b10101010;
        }

        for split_point in 0..buf.len() {
            let (first, second) = buf.split_at(split_point);
            let res = Message::deserialize(first, second);
            if (6..47).contains(&split_point) {
                assert!(
                    matches!(res, Err(MessageDeserializeError::NeedContiguous)),
                    "expected NeedContiguous: split_point={split_point}"
                );
                continue;
            }
            let (msg, len) = res.context(split_point).unwrap();
            let bf = match &msg {
                Message::Bitfield(bf) => bf,
                other => panic!("expected bitfield, got {other:?}"),
            };
            assert_eq!(len, 47);
            assert_eq!(bf.as_ref().len(), 42);
            for byte in bf.as_ref() {
                assert_eq!(*byte, 0b10101010);
            }
            let mut tmp = [0u8; 100];
            let slen = msg.serialize(&mut tmp, &|| Default::default()).unwrap();
            assert_eq!(slen, len);
            assert_eq!(buf[..len], tmp[..len]);
        }
    }

    #[test]
    fn test_no_data_messages() {
        let mut buf = [0u8; 100];

        for msgid in [
            MSGID_CHOKE,
            MSGID_UNCHOKE,
            MSGID_INTERESTED,
            MSGID_NOT_INTERESTED,
        ] {
            buf[0..4].copy_from_slice(&1u32.to_be_bytes());
            buf[4] = msgid;
            for split_point in 0..buf.len() {
                let (first, second) = buf.split_at(split_point);
                let (msg, len) = Message::deserialize(first, second).unwrap();
                match (msgid, &msg) {
                    (MSGID_CHOKE, Message::Choke)
                    | (MSGID_UNCHOKE, Message::Unchoke)
                    | (MSGID_INTERESTED, Message::Interested)
                    | (MSGID_NOT_INTERESTED, Message::NotInterested) => {}
                    (msgid, msg) => panic!("msgid={msgid}, msg={msg:?}"),
                }
                assert_eq!(len, 5);
                let mut tmp = [0u8; 100];
                let slen = msg.serialize(&mut tmp, &|| Default::default()).unwrap();
                assert_eq!(slen, len);
                assert_eq!(buf[..len], tmp[..len]);
            }
        }
    }

    // A bitfield carries one bit per piece, so unlike every other message its
    // length is a property of the torrent. It is the one message that does not
    // fit in MAX_MSG_LEN, and a buffer of that size refuses to hold it just
    // past 131,960 pieces.
    /// BEP 54 `lt_donthave` round trips, and its payload is not bencode.
    ///
    /// Four big-endian bytes and nothing else. Every other extension message
    /// in this crate carries a bencoded body, so a `lt_donthave` that reached
    /// the generic arm would be parsed as bencode and fail. See
    /// TODO/bep-coverage.md T-167.
    #[test]
    fn test_lt_donthave_round_trips() {
        use crate::extended::{ExtendedMessage, PeerExtendedMessageIds};

        let msg = Message::Extended(ExtendedMessage::LtDontHave(70_000));
        let mut buf = vec![0u8; 64];
        let len = msg
            .serialize(&mut buf, &|| PeerExtendedMessageIds::my())
            .unwrap();

        // 4 length prefix, 1 message id, 1 extension id, 4 payload.
        assert_eq!(len, 10);
        assert_eq!(&buf[6..10], &70_000u32.to_be_bytes());
        assert_eq!(buf[5], MY_EXTENDED_LT_DONTHAVE);

        let (de, dlen) = Message::deserialize(&buf[..len], &[]).unwrap();
        assert_eq!(dlen, len);
        match de {
            Message::Extended(ExtendedMessage::LtDontHave(piece)) => assert_eq!(piece, 70_000),
            other => panic!("expected lt_donthave, got {other:?}"),
        }
    }

    /// BEP 6, all five, on the wire and back. The ids are the BEP's and the
    /// lengths are what a peer will send, so a mistake in either shows here
    /// rather than as a dropped connection against a real client.
    #[test]
    fn test_bep6_messages_round_trip() {
        let cases: &[(Message<'_>, MsgId, usize)] = &[
            (Message::HaveAll, MSGID_HAVE_ALL, 5),
            (Message::HaveNone, MSGID_HAVE_NONE, 5),
            (Message::SuggestPiece(1313), MSGID_SUGGEST_PIECE, 9),
            (Message::AllowedFast(1059), MSGID_ALLOWED_FAST, 9),
            (
                Message::RejectRequest(Request {
                    index: 7,
                    begin: 16384,
                    length: 16384,
                }),
                MSGID_REJECT_REQUEST,
                17,
            ),
        ];
        for (msg, msg_id, total) in cases {
            let mut buf = vec![0u8; 64];
            let len = msg.serialize(&mut buf, &Default::default).unwrap();
            assert_eq!(len, *total, "{msg:?}");
            assert_eq!(buf[4], *msg_id, "{msg:?}");
            assert_eq!(
                &buf[..4],
                &((*total as u32) - 4).to_be_bytes(),
                "length prefix for {msg:?}"
            );
            let (de, dlen) = Message::deserialize(&buf[..len], &[]).unwrap();
            assert_eq!(dlen, len, "{msg:?}");
            match (msg, &de) {
                (Message::HaveAll, Message::HaveAll) => {}
                (Message::HaveNone, Message::HaveNone) => {}
                (Message::SuggestPiece(a), Message::SuggestPiece(b)) => assert_eq!(a, b),
                (Message::AllowedFast(a), Message::AllowedFast(b)) => assert_eq!(a, b),
                (Message::RejectRequest(a), Message::RejectRequest(b)) => {
                    assert_eq!(a.index, b.index);
                    assert_eq!(a.begin, b.begin);
                    assert_eq!(a.length, b.length);
                }
                (sent, got) => panic!("sent {sent:?}, got {got:?}"),
            }
        }
    }

    /// `reject request` shares its body with `request` and `cancel` and must
    /// not be confused with either. Three ids, three variants.
    #[test]
    fn test_reject_request_is_not_a_request() {
        let request = Request {
            index: 3,
            begin: 0,
            length: 16384,
        };
        for (msg, id) in [
            (Message::Request(request), MSGID_REQUEST),
            (Message::Cancel(request), MSGID_CANCEL),
            (Message::RejectRequest(request), MSGID_REJECT_REQUEST),
        ] {
            let mut buf = vec![0u8; 32];
            let len = msg.serialize(&mut buf, &Default::default).unwrap();
            assert_eq!(buf[4], id);
            let (de, _) = Message::deserialize(&buf[..len], &[]).unwrap();
            let same = match (&msg, &de) {
                (Message::Request(..), Message::Request(..)) => true,
                (Message::Cancel(..), Message::Cancel(..)) => true,
                (Message::RejectRequest(..), Message::RejectRequest(..)) => true,
                _ => false,
            };
            assert!(same, "{msg:?} came back as {de:?}");
        }
    }

    /// The reserved bit is the last byte and `0x04`, which is a different byte
    /// from BEP 10's. Getting the two confused advertises one and negotiates
    /// the other.
    #[test]
    fn test_handshake_advertises_both_reserved_bits() {
        let h = Handshake::new(Id20::new([1u8; 20]), Id20::new([2u8; 20]));
        assert!(h.supports_extended(), "BEP 10");
        assert!(h.supports_fast(), "BEP 6");
        assert_eq!(h.reserved.to_be_bytes()[5], 0x10, "BEP 10 is byte 5");
        assert_eq!(h.reserved.to_be_bytes()[7], 0x04, "BEP 6 is byte 7");

        let mut buf = [0u8; 68];
        let len = h.serialize_unchecked_len(&mut buf);
        let (back, blen) = Handshake::deserialize(&buf[..len]).unwrap();
        assert_eq!(blen, len);
        assert!(back.supports_extended());
        assert!(back.supports_fast());

        // A peer that sets neither is understood as setting neither.
        let plain = Handshake {
            reserved: 0,
            info_hash: Id20::new([1u8; 20]),
            peer_id: Id20::new([2u8; 20]),
        };
        assert!(!plain.supports_extended());
        assert!(!plain.supports_fast());
    }

    /// A peer that never advertised it cannot be sent one.
    #[test]
    fn test_lt_donthave_needs_the_peer_to_have_asked_for_it() {
        use crate::extended::{ExtendedMessage, PeerExtendedMessageIds};

        let msg = Message::Extended(ExtendedMessage::LtDontHave(1));
        let mut buf = vec![0u8; 64];
        assert!(matches!(
            msg.serialize(&mut buf, &PeerExtendedMessageIds::default),
            Err(SerializeError::NeedLtDontHave)
        ));
    }

    /// Not attributed until 2026-08-23. The `#[test]` this needed had landed
    /// on the test above it, which then carried two, so this one was compiled
    /// and never run. It is T-194's own regression test.
    #[test]
    fn test_bitfield_larger_than_max_msg_len() {
        const PIECES: usize = 131_961;
        let bitfield = vec![0xffu8; PIECES.div_ceil(8)];
        let msg = Message::Bitfield(ByteBuf(&bitfield));

        let needed = Message::bitfield_message_len(bitfield.len());
        assert!(
            needed > MAX_MSG_LEN,
            "this test is pointless unless {needed} exceeds {MAX_MSG_LEN}"
        );

        let mut fixed = vec![0u8; MAX_MSG_LEN];
        assert!(matches!(
            msg.serialize(&mut fixed, &|| Default::default()),
            Err(SerializeError::NoSpaceInBuffer)
        ));

        let mut sized = vec![0u8; needed];
        let len = msg.serialize(&mut sized, &|| Default::default()).unwrap();
        assert_eq!(len, needed);

        let (de, dlen) = Message::deserialize(&sized[..len], &[]).unwrap();
        assert_eq!(dlen, len);
        match de {
            Message::Bitfield(b) => assert_eq!(b.as_ref(), &bitfield[..]),
            other => panic!("expected a bitfield, got {other:?}"),
        }
    }
}
