use std::fmt;
use std::io::{Read, Write};

const PSTR: &str = "BitTorrent protocol";
const PSTR_LEN: usize = 19;
const HANDSHAKE_LEN: usize = 49 + PSTR_LEN;
const MAX_MESSAGE_LEN: usize = 2 * 1024 * 1024;
const EXTENSION_PROTOCOL_BIT: u8 = 0x10;
const HYBRID_V2_UPGRADE_BIT: u8 = 0x10;
const HASH_REQUEST_PAYLOAD_LEN: usize = 48;
const MAX_HASH_REQUEST_LENGTH: u32 = 512;
const MAX_HASH_TREE_LAYERS: u32 = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handshake {
    pub reserved: [u8; 8],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
}

impl Handshake {
    pub fn supports_extensions(&self) -> bool {
        self.reserved[5] & EXTENSION_PROTOCOL_BIT != 0
    }

    /// BEP 52's upgrade signal: the fourth most-significant bit in the final
    /// reserved byte of a v1 hybrid-torrent handshake.
    pub fn supports_hybrid_v2_upgrade(&self) -> bool {
        self.reserved[7] & HYBRID_V2_UPGRADE_BIT != 0
    }
}

/// The fixed request tuple shared by BEP 52 hash-request and hash-reject
/// messages. All integer fields are encoded as four-byte big-endian values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HashRequest {
    pub pieces_root: [u8; 32],
    pub base_layer: u32,
    pub index: u32,
    pub length: u32,
    pub proof_layers: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u32),
    Bitfield(Vec<u8>),
    Request {
        index: u32,
        begin: u32,
        length: u32,
    },
    Piece {
        index: u32,
        begin: u32,
        block: Vec<u8>,
    },
    Cancel {
        index: u32,
        begin: u32,
        length: u32,
    },
    Port(u16),
    Extended {
        ext_id: u8,
        payload: Vec<u8>,
    },
    // BEP 6 - Fast Extension
    SuggestPiece(u32),
    HaveAll,
    HaveNone,
    RejectRequest {
        index: u32,
        begin: u32,
        length: u32,
    },
    AllowedFast(u32),
    // BEP 52 - v2 Merkle hash exchange.
    HashRequest(HashRequest),
    Hashes {
        request: HashRequest,
        hashes: Vec<[u8; 32]>,
    },
    HashReject(HashRequest),
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    InvalidHandshake,
    InvalidProtocol,
    InvalidMessage,
    InvalidLength,
    UnsupportedMessage(u8),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "io error: {err}"),
            Error::InvalidHandshake => write!(f, "invalid handshake"),
            Error::InvalidProtocol => write!(f, "invalid protocol string"),
            Error::InvalidMessage => write!(f, "invalid message"),
            Error::InvalidLength => write!(f, "invalid message length"),
            Error::UnsupportedMessage(id) => write!(f, "unsupported message id {id}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

pub fn build_handshake(
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    extensions: bool,
) -> [u8; HANDSHAKE_LEN] {
    build_handshake_with_hybrid_upgrade(info_hash, peer_id, extensions, false)
}

pub fn build_handshake_with_hybrid_upgrade(
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    extensions: bool,
    hybrid_v2_upgrade: bool,
) -> [u8; HANDSHAKE_LEN] {
    let mut out = [0u8; HANDSHAKE_LEN];
    out[0] = PSTR_LEN as u8;
    out[1..1 + PSTR_LEN].copy_from_slice(PSTR.as_bytes());
    let reserved_start = 1 + PSTR_LEN;
    if extensions {
        out[reserved_start + 5] |= EXTENSION_PROTOCOL_BIT;
    }
    if hybrid_v2_upgrade {
        out[reserved_start + 7] |= HYBRID_V2_UPGRADE_BIT;
    }
    let info_start = reserved_start + 8;
    let peer_start = info_start + 20;
    out[info_start..peer_start].copy_from_slice(&info_hash);
    out[peer_start..peer_start + 20].copy_from_slice(&peer_id);
    out
}

pub fn parse_handshake(bytes: &[u8]) -> Result<Handshake, Error> {
    if bytes.len() != HANDSHAKE_LEN {
        return Err(Error::InvalidHandshake);
    }
    if bytes[0] as usize != PSTR_LEN {
        return Err(Error::InvalidHandshake);
    }
    if &bytes[1..1 + PSTR_LEN] != PSTR.as_bytes() {
        return Err(Error::InvalidProtocol);
    }
    let reserved_start = 1 + PSTR_LEN;
    let info_start = reserved_start + 8;
    let peer_start = info_start + 20;

    let mut reserved = [0u8; 8];
    reserved.copy_from_slice(&bytes[reserved_start..info_start]);
    let mut info_hash = [0u8; 20];
    info_hash.copy_from_slice(&bytes[info_start..peer_start]);
    let mut peer_id = [0u8; 20];
    peer_id.copy_from_slice(&bytes[peer_start..peer_start + 20]);

    Ok(Handshake {
        reserved,
        info_hash,
        peer_id,
    })
}

pub fn write_handshake<W: Write>(
    writer: &mut W,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    extensions: bool,
) -> Result<(), Error> {
    let data = build_handshake(info_hash, peer_id, extensions);
    writer.write_all(&data)?;
    Ok(())
}

pub fn write_handshake_with_hybrid_upgrade<W: Write>(
    writer: &mut W,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    extensions: bool,
    hybrid_v2_upgrade: bool,
) -> Result<(), Error> {
    let data =
        build_handshake_with_hybrid_upgrade(info_hash, peer_id, extensions, hybrid_v2_upgrade);
    writer.write_all(&data)?;
    Ok(())
}

pub fn read_handshake<R: Read>(reader: &mut R) -> Result<Handshake, Error> {
    let mut buf = [0u8; HANDSHAKE_LEN];
    reader.read_exact(&mut buf)?;
    parse_handshake(&buf)
}

pub fn write_message<W: Write>(writer: &mut W, message: &Message) -> Result<(), Error> {
    if encoded_payload_len(message).ok_or(Error::InvalidLength)? > MAX_MESSAGE_LEN {
        return Err(Error::InvalidLength);
    }
    let data = encode_message(message);
    writer.write_all(&data)?;
    Ok(())
}

fn encoded_payload_len(message: &Message) -> Option<usize> {
    match message {
        Message::KeepAlive => Some(0),
        Message::Choke
        | Message::Unchoke
        | Message::Interested
        | Message::NotInterested
        | Message::HaveAll
        | Message::HaveNone => Some(1),
        Message::Have(_) | Message::SuggestPiece(_) | Message::AllowedFast(_) => Some(5),
        Message::Bitfield(bits) => 1usize.checked_add(bits.len()),
        Message::Request { .. } | Message::Cancel { .. } | Message::RejectRequest { .. } => {
            Some(13)
        }
        Message::Piece { block, .. } => 9usize.checked_add(block.len()),
        Message::Port(_) => Some(3),
        Message::Extended { payload, .. } => 2usize.checked_add(payload.len()),
        Message::HashRequest(request) | Message::HashReject(request) => {
            validate_hash_request(request).then_some(1 + HASH_REQUEST_PAYLOAD_LEN)
        }
        Message::Hashes { request, hashes } => {
            if !validate_hashes(request, hashes) {
                return None;
            }
            (1 + HASH_REQUEST_PAYLOAD_LEN).checked_add(hashes.len().checked_mul(32)?)
        }
    }
}

#[allow(dead_code)]
pub fn read_message<R: Read>(reader: &mut R) -> Result<Message, Error> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 {
        return Ok(Message::KeepAlive);
    }
    if len > MAX_MESSAGE_LEN {
        return Err(Error::InvalidLength);
    }
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload)?;
    decode_message(&payload)
}

pub struct MessageReader {
    buf: Vec<u8>,
    start: usize,
}

impl MessageReader {
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(64 * 1024),
            start: 0,
        }
    }

    pub fn read_message<R: Read>(&mut self, reader: &mut R) -> Result<Option<Message>, Error> {
        if let Some(message) = self.try_parse()? {
            return Ok(Some(message));
        }

        // Perform at most one socket read per call. A peer that supplies a
        // partial frame one byte at a time must not keep this function inside
        // an unbounded progress loop and prevent its caller from observing a
        // stop request or an absolute operation deadline.
        let mut tmp = [0u8; 4096];
        match reader.read(&mut tmp) {
            Ok(0) => Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "peer closed connection",
            ))),
            Ok(n) => {
                self.buf.extend_from_slice(&tmp[..n]);
                self.try_parse()
            }
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                Ok(None)
            }
            Err(err) => Err(Error::Io(err)),
        }
    }

    fn try_parse(&mut self) -> Result<Option<Message>, Error> {
        if self.available() < 4 {
            return Ok(None);
        }
        let header = &self.buf[self.start..self.start + 4];
        let len = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        if len > MAX_MESSAGE_LEN {
            return Err(Error::InvalidLength);
        }
        let total = 4 + len;
        if self.available() < total {
            return Ok(None);
        }
        if len == 0 {
            self.consume(4);
            return Ok(Some(Message::KeepAlive));
        }
        let payload_start = self.start + 4;
        let payload_end = payload_start + len;
        let payload = self.buf[payload_start..payload_end].to_vec();
        self.consume(total);
        Ok(Some(decode_message(&payload)?))
    }

    fn available(&self) -> usize {
        self.buf.len().saturating_sub(self.start)
    }

    fn consume(&mut self, amount: usize) {
        self.start = self.start.saturating_add(amount);
        if self.start == self.buf.len() {
            self.buf.clear();
            self.start = 0;
        } else if self.start >= 64 * 1024 {
            let remaining = self.buf.len() - self.start;
            self.buf.copy_within(self.start.., 0);
            self.buf.truncate(remaining);
            self.start = 0;
        }
    }
}

pub fn encode_message(message: &Message) -> Vec<u8> {
    match message {
        Message::KeepAlive => vec![0, 0, 0, 0],
        Message::Choke => encode_simple(0),
        Message::Unchoke => encode_simple(1),
        Message::Interested => encode_simple(2),
        Message::NotInterested => encode_simple(3),
        Message::Have(index) => {
            let mut payload = Vec::with_capacity(5);
            payload.push(4);
            payload.extend_from_slice(&index.to_be_bytes());
            with_len_prefix(payload)
        }
        Message::Bitfield(bits) => {
            let mut payload = Vec::with_capacity(1 + bits.len());
            payload.push(5);
            payload.extend_from_slice(bits);
            with_len_prefix(payload)
        }
        Message::Request {
            index,
            begin,
            length,
        } => encode_triple(6, *index, *begin, *length),
        Message::Piece {
            index,
            begin,
            block,
        } => {
            let mut payload = Vec::with_capacity(9 + block.len());
            payload.push(7);
            payload.extend_from_slice(&index.to_be_bytes());
            payload.extend_from_slice(&begin.to_be_bytes());
            payload.extend_from_slice(block);
            with_len_prefix(payload)
        }
        Message::Cancel {
            index,
            begin,
            length,
        } => encode_triple(8, *index, *begin, *length),
        Message::Port(port) => {
            let mut payload = Vec::with_capacity(3);
            payload.push(9);
            payload.extend_from_slice(&port.to_be_bytes());
            with_len_prefix(payload)
        }
        Message::Extended { ext_id, payload } => {
            let mut buf = Vec::with_capacity(2 + payload.len());
            buf.push(20);
            buf.push(*ext_id);
            buf.extend_from_slice(payload);
            with_len_prefix(buf)
        }
        // BEP 6 - Fast Extension
        Message::SuggestPiece(index) => {
            let mut payload = Vec::with_capacity(5);
            payload.push(13);
            payload.extend_from_slice(&index.to_be_bytes());
            with_len_prefix(payload)
        }
        Message::HaveAll => encode_simple(14),
        Message::HaveNone => encode_simple(15),
        Message::RejectRequest {
            index,
            begin,
            length,
        } => encode_triple(16, *index, *begin, *length),
        Message::AllowedFast(index) => {
            let mut payload = Vec::with_capacity(5);
            payload.push(17);
            payload.extend_from_slice(&index.to_be_bytes());
            with_len_prefix(payload)
        }
        Message::HashRequest(request) => encode_hash_request(21, request),
        Message::Hashes { request, hashes } => {
            let mut payload = encode_hash_request_payload(22, request);
            for hash in hashes {
                payload.extend_from_slice(hash);
            }
            with_len_prefix(payload)
        }
        Message::HashReject(request) => encode_hash_request(23, request),
    }
}

pub fn decode_message(payload: &[u8]) -> Result<Message, Error> {
    if payload.is_empty() {
        return Err(Error::InvalidMessage);
    }
    let id = payload[0];
    let data = &payload[1..];
    match id {
        0 => expect_empty(data, Message::Choke),
        1 => expect_empty(data, Message::Unchoke),
        2 => expect_empty(data, Message::Interested),
        3 => expect_empty(data, Message::NotInterested),
        4 => {
            if data.len() != 4 {
                return Err(Error::InvalidMessage);
            }
            Ok(Message::Have(read_u32(data)?))
        }
        5 => Ok(Message::Bitfield(data.to_vec())),
        6 => decode_triple(data, |index, begin, length| Message::Request {
            index,
            begin,
            length,
        }),
        7 => {
            if data.len() < 8 {
                return Err(Error::InvalidMessage);
            }
            let index = read_u32(&data[0..4])?;
            let begin = read_u32(&data[4..8])?;
            let block = data[8..].to_vec();
            Ok(Message::Piece {
                index,
                begin,
                block,
            })
        }
        8 => decode_triple(data, |index, begin, length| Message::Cancel {
            index,
            begin,
            length,
        }),
        9 => {
            if data.len() != 2 {
                return Err(Error::InvalidMessage);
            }
            Ok(Message::Port(u16::from_be_bytes([data[0], data[1]])))
        }
        // BEP 6 - Fast Extension
        13 => {
            if data.len() != 4 {
                return Err(Error::InvalidMessage);
            }
            Ok(Message::SuggestPiece(read_u32(data)?))
        }
        14 => expect_empty(data, Message::HaveAll),
        15 => expect_empty(data, Message::HaveNone),
        16 => decode_triple(data, |index, begin, length| Message::RejectRequest {
            index,
            begin,
            length,
        }),
        17 => {
            if data.len() != 4 {
                return Err(Error::InvalidMessage);
            }
            Ok(Message::AllowedFast(read_u32(data)?))
        }
        20 => {
            if data.is_empty() {
                return Err(Error::InvalidMessage);
            }
            Ok(Message::Extended {
                ext_id: data[0],
                payload: data[1..].to_vec(),
            })
        }
        21 => Ok(Message::HashRequest(decode_hash_request(data)?)),
        22 => decode_hashes(data),
        23 => Ok(Message::HashReject(decode_hash_request(data)?)),
        other => Err(Error::UnsupportedMessage(other)),
    }
}

fn encode_hash_request(id: u8, request: &HashRequest) -> Vec<u8> {
    with_len_prefix(encode_hash_request_payload(id, request))
}

fn encode_hash_request_payload(id: u8, request: &HashRequest) -> Vec<u8> {
    let mut payload = Vec::with_capacity(1 + HASH_REQUEST_PAYLOAD_LEN);
    payload.push(id);
    payload.extend_from_slice(&request.pieces_root);
    payload.extend_from_slice(&request.base_layer.to_be_bytes());
    payload.extend_from_slice(&request.index.to_be_bytes());
    payload.extend_from_slice(&request.length.to_be_bytes());
    payload.extend_from_slice(&request.proof_layers.to_be_bytes());
    payload
}

fn decode_hash_request(data: &[u8]) -> Result<HashRequest, Error> {
    if data.len() != HASH_REQUEST_PAYLOAD_LEN {
        return Err(Error::InvalidMessage);
    }
    let mut pieces_root = [0u8; 32];
    pieces_root.copy_from_slice(&data[..32]);
    let request = HashRequest {
        pieces_root,
        base_layer: read_u32(&data[32..36])?,
        index: read_u32(&data[36..40])?,
        length: read_u32(&data[40..44])?,
        proof_layers: read_u32(&data[44..48])?,
    };
    if !validate_hash_request(&request) {
        return Err(Error::InvalidMessage);
    }
    Ok(request)
}

fn decode_hashes(data: &[u8]) -> Result<Message, Error> {
    if data.len() < HASH_REQUEST_PAYLOAD_LEN
        || !(data.len() - HASH_REQUEST_PAYLOAD_LEN).is_multiple_of(32)
    {
        return Err(Error::InvalidMessage);
    }
    let request = decode_hash_request(&data[..HASH_REQUEST_PAYLOAD_LEN])?;
    let mut hashes = Vec::with_capacity((data.len() - HASH_REQUEST_PAYLOAD_LEN) / 32);
    for chunk in data[HASH_REQUEST_PAYLOAD_LEN..].chunks_exact(32) {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(chunk);
        hashes.push(hash);
    }
    if !validate_hashes(&request, &hashes) {
        return Err(Error::InvalidMessage);
    }
    Ok(Message::Hashes { request, hashes })
}

fn validate_hash_request(request: &HashRequest) -> bool {
    request.base_layer <= MAX_HASH_TREE_LAYERS
        && request.proof_layers <= MAX_HASH_TREE_LAYERS
        && request.length >= 2
        && request.length <= MAX_HASH_REQUEST_LENGTH
        && request.length.is_power_of_two()
        && request.index.is_multiple_of(request.length)
        && request.index.checked_add(request.length).is_some()
}

fn validate_hashes(request: &HashRequest, hashes: &[[u8; 32]]) -> bool {
    if !validate_hash_request(request) {
        return false;
    }
    let Ok(base_hashes) = usize::try_from(request.length) else {
        return false;
    };
    let Ok(max_proofs) = usize::try_from(request.proof_layers) else {
        return false;
    };
    hashes.len() >= base_hashes && hashes.len() <= base_hashes.saturating_add(max_proofs)
}

fn encode_simple(id: u8) -> Vec<u8> {
    with_len_prefix(vec![id])
}

fn encode_triple(id: u8, first: u32, second: u32, third: u32) -> Vec<u8> {
    let mut payload = Vec::with_capacity(13);
    payload.push(id);
    payload.extend_from_slice(&first.to_be_bytes());
    payload.extend_from_slice(&second.to_be_bytes());
    payload.extend_from_slice(&third.to_be_bytes());
    with_len_prefix(payload)
}

fn with_len_prefix(mut payload: Vec<u8>) -> Vec<u8> {
    let len = payload.len() as u32;
    let mut out = Vec::with_capacity(payload.len() + 4);
    out.extend_from_slice(&len.to_be_bytes());
    out.append(&mut payload);
    out
}

fn expect_empty(data: &[u8], msg: Message) -> Result<Message, Error> {
    if !data.is_empty() {
        return Err(Error::InvalidMessage);
    }
    Ok(msg)
}

fn decode_triple<F>(data: &[u8], build: F) -> Result<Message, Error>
where
    F: Fn(u32, u32, u32) -> Message,
{
    if data.len() != 12 {
        return Err(Error::InvalidMessage);
    }
    let first = read_u32(&data[0..4])?;
    let second = read_u32(&data[4..8])?;
    let third = read_u32(&data[8..12])?;
    Ok(build(first, second, third))
}

fn read_u32(bytes: &[u8]) -> Result<u32, Error> {
    if bytes.len() != 4 {
        return Err(Error::InvalidMessage);
    }
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};

    struct OneByteReader {
        bytes: Vec<u8>,
        offset: usize,
        reads: usize,
    }

    impl Read for OneByteReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.reads += 1;
            if self.offset == self.bytes.len() {
                return Ok(0);
            }
            buf[0] = self.bytes[self.offset];
            self.offset += 1;
            Ok(1)
        }
    }

    struct SingleChunkReader {
        bytes: Option<Vec<u8>>,
        reads: usize,
    }

    impl Read for SingleChunkReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.reads += 1;
            let bytes = self
                .bytes
                .take()
                .expect("buffered message parsing performed an extra read");
            buf[..bytes.len()].copy_from_slice(&bytes);
            Ok(bytes.len())
        }
    }

    #[test]
    fn handshake_roundtrip() {
        let info_hash = [1u8; 20];
        let peer_id = [2u8; 20];
        let bytes = build_handshake(info_hash, peer_id, true);
        let parsed = parse_handshake(&bytes).unwrap();
        assert_eq!(parsed.info_hash, info_hash);
        assert_eq!(parsed.peer_id, peer_id);
        assert!(parsed.supports_extensions());
        assert!(!parsed.supports_hybrid_v2_upgrade());
    }

    #[test]
    fn hybrid_upgrade_handshake_sets_only_the_bep52_reserved_bit() {
        let bytes = build_handshake_with_hybrid_upgrade([1u8; 20], [2u8; 20], true, true);
        let parsed = parse_handshake(&bytes).unwrap();
        assert!(parsed.supports_extensions());
        assert!(parsed.supports_hybrid_v2_upgrade());
        assert_eq!(parsed.reserved, [0, 0, 0, 0, 0, 0x10, 0, 0x10]);
    }

    #[test]
    fn message_roundtrip() {
        let hash_request = HashRequest {
            pieces_root: [9u8; 32],
            base_layer: 2,
            index: 0,
            length: 2,
            proof_layers: 3,
        };
        let messages = vec![
            Message::KeepAlive,
            Message::Choke,
            Message::Interested,
            Message::Have(42),
            Message::Request {
                index: 1,
                begin: 2,
                length: 3,
            },
            Message::Piece {
                index: 4,
                begin: 8,
                block: vec![1, 2, 3, 4],
            },
            Message::Extended {
                ext_id: 2,
                payload: b"hello".to_vec(),
            },
            Message::HashRequest(hash_request),
            Message::Hashes {
                request: hash_request,
                hashes: vec![[1u8; 32], [2u8; 32], [3u8; 32]],
            },
            Message::HashReject(hash_request),
        ];

        for msg in messages {
            let data = encode_message(&msg);
            let mut cursor = Cursor::new(data);
            let decoded = read_message(&mut cursor).unwrap();
            assert_eq!(decoded, msg);
        }
    }

    #[test]
    fn parse_handshake_rejects_wrong_protocol() {
        let mut bytes = build_handshake([1u8; 20], [2u8; 20], false);
        bytes[1] = b'X';
        assert!(matches!(
            parse_handshake(&bytes),
            Err(Error::InvalidProtocol)
        ));
    }

    #[test]
    fn read_message_rejects_oversized_length() {
        let len = (MAX_MESSAGE_LEN as u32).saturating_add(1);
        let data = len.to_be_bytes();
        let mut cursor = Cursor::new(data);
        assert!(matches!(
            read_message(&mut cursor),
            Err(Error::InvalidLength)
        ));
    }

    #[test]
    fn write_message_rejects_oversized_payload() {
        let message = Message::Extended {
            ext_id: 1,
            payload: vec![0; MAX_MESSAGE_LEN],
        };
        let mut out = Vec::new();
        assert!(matches!(
            write_message(&mut out, &message),
            Err(Error::InvalidLength)
        ));
        assert!(out.is_empty());
    }

    #[test]
    fn hash_messages_use_bep52_ids_and_fixed_header() {
        let request = HashRequest {
            pieces_root: [0xabu8; 32],
            base_layer: 4,
            index: 8,
            length: 4,
            proof_layers: 5,
        };
        let request_bytes = encode_message(&Message::HashRequest(request));
        assert_eq!(
            u32::from_be_bytes(request_bytes[..4].try_into().unwrap()),
            49
        );
        assert_eq!(request_bytes[4], 21);
        assert_eq!(&request_bytes[5..37], &[0xabu8; 32]);

        let hashes = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];
        let hashes_bytes = encode_message(&Message::Hashes {
            request,
            hashes: hashes.clone(),
        });
        assert_eq!(hashes_bytes[4], 22);
        assert_eq!(hashes_bytes.len(), 4 + 49 + hashes.len() * 32);

        let reject_bytes = encode_message(&Message::HashReject(request));
        assert_eq!(reject_bytes[4], 23);
    }

    #[test]
    fn hash_message_decoder_enforces_request_and_response_bounds() {
        let request = HashRequest {
            pieces_root: [7u8; 32],
            base_layer: 0,
            index: 0,
            length: 2,
            proof_layers: 1,
        };

        let mut invalid_length = encode_hash_request_payload(21, &request);
        invalid_length[41..45].copy_from_slice(&513u32.to_be_bytes());
        assert!(matches!(
            decode_message(&invalid_length),
            Err(Error::InvalidMessage)
        ));

        let mut misaligned = encode_hash_request_payload(21, &request);
        misaligned[37..41].copy_from_slice(&1u32.to_be_bytes());
        assert!(matches!(
            decode_message(&misaligned),
            Err(Error::InvalidMessage)
        ));

        let too_few_hashes = encode_hash_request_payload(22, &request);
        assert!(matches!(
            decode_message(&too_few_hashes),
            Err(Error::InvalidMessage)
        ));

        let too_many_hashes = Message::Hashes {
            request,
            hashes: vec![[0u8; 32]; 4],
        };
        let mut out = Vec::new();
        assert!(matches!(
            write_message(&mut out, &too_many_hashes),
            Err(Error::InvalidLength)
        ));
        assert!(out.is_empty());
    }

    #[test]
    fn decode_message_rejects_unsupported_id() {
        assert!(matches!(
            decode_message(&[99]),
            Err(Error::UnsupportedMessage(99))
        ));
    }

    #[test]
    fn message_reader_parses_incremental_frames() {
        let mut reader = MessageReader::new();
        reader
            .buf
            .extend_from_slice(&encode_message(&Message::KeepAlive));
        reader
            .buf
            .extend_from_slice(&encode_message(&Message::Have(7)));

        let first = reader.try_parse().unwrap();
        let second = reader.try_parse().unwrap();
        let third = reader.try_parse().unwrap();

        assert_eq!(first, Some(Message::KeepAlive));
        assert_eq!(second, Some(Message::Have(7)));
        assert_eq!(third, None);
    }

    #[test]
    fn message_reader_returns_after_each_slow_trickle_read() {
        let mut source = OneByteReader {
            bytes: encode_message(&Message::Interested),
            offset: 0,
            reads: 0,
        };
        let mut reader = MessageReader::new();

        for expected_reads in 1..=4 {
            assert_eq!(reader.read_message(&mut source).unwrap(), None);
            assert_eq!(source.reads, expected_reads);
        }
        assert_eq!(
            reader.read_message(&mut source).unwrap(),
            Some(Message::Interested)
        );
        assert_eq!(source.reads, 5);
    }

    #[test]
    fn message_reader_parses_buffered_followup_without_another_read() {
        let mut bytes = encode_message(&Message::KeepAlive);
        bytes.extend_from_slice(&encode_message(&Message::Have(11)));
        let mut source = SingleChunkReader {
            bytes: Some(bytes),
            reads: 0,
        };
        let mut reader = MessageReader::new();

        assert_eq!(
            reader.read_message(&mut source).unwrap(),
            Some(Message::KeepAlive)
        );
        assert_eq!(
            reader.read_message(&mut source).unwrap(),
            Some(Message::Have(11))
        );
        assert_eq!(source.reads, 1);
    }
}
