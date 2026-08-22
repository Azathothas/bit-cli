use std::io::{Read, Write};

use num_bigint::BigUint;
use num_traits::Num;

use crate::sha1;

pub enum CryptoMode {
    Plaintext,
    Rc4,
}

pub type AcceptOutcome = (CryptoMode, Option<CipherState>, [u8; 20], Vec<u8>, Vec<u8>);

#[derive(Clone)]
pub struct CipherState {
    enc: Rc4,
    dec: Rc4,
}

impl CipherState {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new(enc_key: &[u8], dec_key: &[u8]) -> Self {
        let mut enc = Rc4::new(enc_key);
        let mut dec = Rc4::new(dec_key);
        enc.discard(1024);
        dec.discard(1024);
        Self { enc, dec }
    }

    pub fn encrypt(&mut self, data: &mut [u8]) {
        self.enc.apply(data);
    }

    pub fn decrypt(&mut self, data: &mut [u8]) {
        self.dec.apply(data);
    }
}

/// MSE/PE initiator handshake (outbound connection).
///
/// Follows the standard BEP MSE/PE protocol:
///   Step 1: Send Ya (DH public key)
///   Step 2: Read Yb (peer's DH public key)
///   Step 3: Send HASH('req1', S) + XOR'd hash + ENCRYPT(VC, crypto_provide, PadC, len(IA), IA)
///   Step 4: Read peer's ENCRYPT(VC, crypto_select, PadD)
///
/// `initial_payload` is the BT handshake (68 bytes) sent as IA in step 3.
/// After this returns, the peer's BT handshake is the next thing in the encrypted stream.
pub fn initiate<RW: Read + Write>(
    stream: &mut RW,
    info_hash: [u8; 20],
    allow_plain: bool,
    initial_payload: &[u8],
) -> Result<(CryptoMode, Option<CipherState>, Vec<u8>), String> {
    let ia_len = u16::try_from(initial_payload.len())
        .map_err(|_| "mse initial payload too large".to_string())?;
    // Step 1: Send Ya (96 bytes, no padding)
    let (priv_key, pub_key) = dh_generate()?;
    let pub_bytes = to_fixed_bytes(&pub_key, 96);
    stream
        .write_all(&pub_bytes)
        .map_err(|err| err.to_string())?;

    // Step 2: Read Yb (96 bytes)
    let mut peer_pub = [0u8; 96];
    stream
        .read_exact(&mut peer_pub)
        .map_err(|err| err.to_string())?;
    let peer_pub = BigUint::from_bytes_be(&peer_pub);
    validate_peer_public(&peer_pub)?;

    // Compute shared secret S, padded to 96 bytes
    let shared = peer_pub.modpow(&priv_key, &prime()?);
    let shared_bytes = to_fixed_bytes(&shared, 96);

    // Compute identification hashes
    let hash_req1 = sha1_bytes(b"req1", &shared_bytes);
    let hash_req2 = sha1_bytes(b"req2", &info_hash);
    let hash_req3 = sha1_bytes(b"req3", &shared_bytes);
    let xor = xor_hash(&hash_req2, &hash_req3);

    // Derive RC4 keys: initiator encrypts with keyA, decrypts with keyB
    let (enc_key, dec_key) = derive_keys(&shared_bytes, &info_hash, true);

    // Set up encryption stream (initiator outgoing)
    let mut enc = Rc4::new(&enc_key);
    enc.discard(1024);

    // Build step 3 encrypted portion: VC + crypto_provide + len(PadC) + len(IA) + IA
    let mut provide = 0x02u32; // RC4
    if allow_plain {
        provide |= 0x01; // also offer plaintext
    }
    let mut enc_data = Vec::with_capacity(8 + 4 + 2 + 2 + initial_payload.len());
    enc_data.extend_from_slice(&[0u8; 8]); // VC (verification constant)
    enc_data.extend_from_slice(&provide.to_be_bytes()); // crypto_provide
    enc_data.extend_from_slice(&0u16.to_be_bytes()); // len(PadC) = 0
    enc_data.extend_from_slice(&ia_len.to_be_bytes()); // len(IA)
    enc_data.extend_from_slice(initial_payload); // IA
    enc.apply(&mut enc_data);

    // Send step 3: plaintext hashes + encrypted data
    stream
        .write_all(&hash_req1)
        .and_then(|_| stream.write_all(&xor))
        .and_then(|_| stream.write_all(&enc_data))
        .map_err(|err| err.to_string())?;

    // Step 4: Scan for peer's encrypted VC
    // The peer may have sent PadB after Yb, so we scan up to 520 bytes.
    // Compute what encrypted VC looks like: RC4 keystream XOR'd with 8 zeros
    let mut vc_pattern = [0u8; 8];
    {
        let mut dec_preview = Rc4::new(&dec_key);
        dec_preview.discard(1024);
        dec_preview.apply(&mut vc_pattern);
    }

    // Read in chunks and search for the VC pattern
    let mut scan_buf = Vec::with_capacity(528);
    let max_scan = 520; // 512 max PadB + 8 VC
    let vc_offset;
    let mut chunk = [0u8; 512];
    'vc_scan: loop {
        let n = stream
            .read(&mut chunk)
            .map_err(|err| format!("mse vc scan: {err}"))?;
        if n == 0 {
            return Err("mse vc scan: unexpected eof".to_string());
        }
        scan_buf.extend_from_slice(&chunk[..n]);
        // Check all new positions where the pattern could start
        let search_start = if scan_buf.len() - n < 8 {
            0
        } else {
            scan_buf.len() - n - 7 // pattern could straddle the boundary
        };
        if let Some(pos) = find_sync_pattern(&scan_buf, &vc_pattern, search_start, 512) {
            vc_offset = pos;
            break 'vc_scan;
        }
        if scan_buf.len() > max_scan {
            return Err("mse vc sync failed".to_string());
        }
    }

    // Set up decryption stream positioned after VC
    let mut dec = Rc4::new(&dec_key);
    dec.discard(1024 + 8); // skip past VC

    // We may have over-read bytes after VC into scan_buf. Keep those bytes raw
    // until crypto_select is known: only the response header and PadD are
    // necessarily encrypted. Bytes following PadD use the selected mode.
    let mut buffered = scan_buf[vc_offset + 8..].to_vec();
    let mut read_exact_buffered = |out: &mut [u8], context: &str| -> Result<(), String> {
        let take = out.len().min(buffered.len());
        if take > 0 {
            out[..take].copy_from_slice(&buffered[..take]);
            buffered.drain(..take);
        }
        if take < out.len() {
            stream
                .read_exact(&mut out[take..])
                .map_err(|err| format!("{context}: {err}"))?;
        }
        Ok(())
    };

    // Read crypto_select (4) + len(PadD) (2). This header is always RC4
    // encrypted, even when the selected payload mode is plaintext.
    let mut header_buf = [0u8; 6];
    read_exact_buffered(&mut header_buf, "mse read header")?;
    dec.apply(&mut header_buf);

    let crypto_select =
        u32::from_be_bytes([header_buf[0], header_buf[1], header_buf[2], header_buf[3]]);
    let pad_d_len = u16::from_be_bytes([header_buf[4], header_buf[5]]) as usize;
    if pad_d_len > 512 {
        return Err("mse PadD too large".to_string());
    }

    // PadD is also always encrypted. Consume exactly PadD and leave any
    // over-read payload bytes in their original representation for now.
    if pad_d_len > 0 {
        let mut pad = vec![0u8; pad_d_len];
        read_exact_buffered(&mut pad, "mse read padD")?;
        dec.apply(&mut pad);
    }

    if crypto_select == 0x02 {
        // PeerStream buffers hold plaintext, so decrypt an over-read encrypted
        // payload and advance the stream cipher by the same amount.
        dec.apply(&mut buffered);
        let cipher = CipherState { enc, dec };
        return Ok((CryptoMode::Rc4, Some(cipher), buffered));
    }
    if allow_plain && crypto_select == 0x01 {
        // The selected plaintext stream starts immediately after PadD. Do not
        // run over-read payload bytes through RC4.
        return Ok((CryptoMode::Plaintext, None, buffered));
    }
    Err("mse crypto selection failed".to_string())
}

/// MSE/PE responder handshake (inbound connection).
///
/// `first_byte` is the first byte already read from the stream (to distinguish
/// plaintext vs MSE). Returns the matched info_hash and the peer's initial
/// payload (BT handshake). After this returns, the caller should send their
/// BT handshake directly through the encrypted stream.
pub fn accept<RW: Read + Write>(
    stream: &mut RW,
    info_hashes: &[[u8; 20]],
    first_byte: u8,
    allow_plain: bool,
) -> Result<AcceptOutcome, String> {
    // Read Ya (first byte already consumed)
    let mut peer_pub = [0u8; 96];
    peer_pub[0] = first_byte;
    stream
        .read_exact(&mut peer_pub[1..])
        .map_err(|err| err.to_string())?;
    let peer_pub = BigUint::from_bytes_be(&peer_pub);

    // Send Yb (no padding)
    validate_peer_public(&peer_pub)?;

    let (priv_key, pub_key) = dh_generate()?;
    let pub_bytes = to_fixed_bytes(&pub_key, 96);
    stream
        .write_all(&pub_bytes)
        .map_err(|err| err.to_string())?;

    // Compute shared secret S, padded to 96 bytes
    let shared = peer_pub.modpow(&priv_key, &prime()?);
    let shared_bytes = to_fixed_bytes(&shared, 96);

    let hash_req1 = sha1_bytes(b"req1", &shared_bytes);
    let hash_req3 = sha1_bytes(b"req3", &shared_bytes);

    // Scan for HASH('req1', S) in stream (skip PadA from peer)
    let mut scan_buf = Vec::with_capacity(540);
    let max_scan = 532; // 512 max PadA + 20 hash
    let mut chunk = [0u8; 512];
    let req1_offset;
    'req1_scan: loop {
        let n = stream
            .read(&mut chunk)
            .map_err(|err| format!("mse req1 scan: {err}"))?;
        if n == 0 {
            return Err("mse req1 scan: unexpected eof".to_string());
        }
        scan_buf.extend_from_slice(&chunk[..n]);
        // Check all new positions where the pattern could start
        let search_start = if scan_buf.len() - n < 20 {
            0
        } else {
            scan_buf.len() - n - 19 // pattern could straddle the boundary
        };
        if let Some(pos) = find_sync_pattern(&scan_buf, &hash_req1, search_start, 512) {
            req1_offset = pos;
            break 'req1_scan;
        }
        if scan_buf.len() > max_scan {
            return Err("mse req1 sync failed".to_string());
        }
    }

    let mut buffered = scan_buf[req1_offset + 20..].to_vec();
    let mut read_exact_buffered = |out: &mut [u8]| -> Result<(), String> {
        let take = out.len().min(buffered.len());
        if take > 0 {
            out[..take].copy_from_slice(&buffered[..take]);
            buffered.drain(..take);
        }
        if take < out.len() {
            stream
                .read_exact(&mut out[take..])
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    };

    // Read XOR'd hash (20 bytes)
    let mut xor_buf = [0u8; 20];
    read_exact_buffered(&mut xor_buf)?;
    let hash_req2 = xor_hash(&xor_buf, &hash_req3);
    let info_hash = match find_info_hash(info_hashes, &hash_req2) {
        Some(hash) => hash,
        None => return Err("mse unknown info hash".to_string()),
    };

    // Derive keys: responder decrypts with keyA (initiator's enc key)
    let (dec_key, enc_key) = derive_keys(&shared_bytes, &info_hash, true);

    // Set up decryption for initiator's encrypted data
    let mut dec = Rc4::new(&dec_key);
    dec.discard(1024);

    // Read and decrypt: VC (8) + crypto_provide (4) + len(PadC) (2)
    let mut enc_header = [0u8; 14];
    read_exact_buffered(&mut enc_header)?;
    dec.apply(&mut enc_header);

    // Verify VC (first 8 bytes should be zeros)
    if enc_header[..8] != [0u8; 8] {
        return Err("mse vc verification failed".to_string());
    }

    let crypto_provide =
        u32::from_be_bytes([enc_header[8], enc_header[9], enc_header[10], enc_header[11]]);
    let pad_c_len = u16::from_be_bytes([enc_header[12], enc_header[13]]) as usize;

    // Skip PadC
    if pad_c_len > 0 {
        if pad_c_len > 512 {
            return Err("mse PadC too large".to_string());
        }
        let mut pad = vec![0u8; pad_c_len];
        read_exact_buffered(&mut pad)?;
        dec.apply(&mut pad);
    }

    // Read len(IA) (2 bytes)
    let mut ia_len_buf = [0u8; 2];
    read_exact_buffered(&mut ia_len_buf)?;
    dec.apply(&mut ia_len_buf);
    let ia_len = u16::from_be_bytes(ia_len_buf) as usize;

    // Read IA (initial payload = peer's BT handshake)
    let mut ia = vec![0u8; ia_len];
    if ia_len > 0 {
        read_exact_buffered(&mut ia)?;
        dec.apply(&mut ia);
    }

    // Determine crypto_select
    let crypto_select: u32 = if crypto_provide & 0x02 != 0 {
        0x02
    } else if allow_plain && crypto_provide & 0x01 != 0 {
        0x01
    } else {
        return Err("mse no compatible crypto".to_string());
    };

    // Set up encryption for our outgoing data and send step 4 response
    let mut enc = Rc4::new(&enc_key);
    enc.discard(1024);

    let mut resp_data = Vec::with_capacity(14);
    resp_data.extend_from_slice(&[0u8; 8]); // VC
    resp_data.extend_from_slice(&crypto_select.to_be_bytes());
    resp_data.extend_from_slice(&0u16.to_be_bytes()); // len(PadD) = 0
    enc.apply(&mut resp_data);
    stream
        .write_all(&resp_data)
        .map_err(|err| err.to_string())?;

    if crypto_select & 0x02 != 0 {
        // PeerStream buffers hold plaintext. Only an RC4-selected stream has
        // encrypted bytes following IA; plaintext-selected bytes must remain
        // exactly as received.
        dec.apply(&mut buffered);
        // Note: dec is for decrypting initiator's data (continuing from IA),
        // enc is for encrypting our data (continuing from step 4 response)
        let cipher = CipherState { enc, dec };
        return Ok((CryptoMode::Rc4, Some(cipher), info_hash, ia, buffered));
    }

    Ok((CryptoMode::Plaintext, None, info_hash, ia, buffered))
}

fn find_sync_pattern(
    data: &[u8],
    pattern: &[u8],
    search_start: usize,
    max_offset: usize,
) -> Option<usize> {
    if pattern.is_empty() || data.len() < pattern.len() {
        return None;
    }
    let last = data.len().checked_sub(pattern.len())?.min(max_offset);
    if search_start > last {
        return None;
    }
    (search_start..=last).find(|offset| data[*offset..*offset + pattern.len()] == *pattern)
}

fn find_info_hash(info_hashes: &[[u8; 20]], target: &[u8; 20]) -> Option<[u8; 20]> {
    info_hashes
        .iter()
        .find(|hash| &sha1_bytes(b"req2", hash.as_slice()) == target)
        .copied()
}

fn sha1_bytes(prefix: &[u8], data: &[u8]) -> [u8; 20] {
    let mut buf = Vec::with_capacity(prefix.len() + data.len());
    buf.extend_from_slice(prefix);
    buf.extend_from_slice(data);
    sha1::sha1(&buf)
}

fn xor_hash(a: &[u8], b: &[u8]) -> [u8; 20] {
    let mut out = [0u8; 20];
    for i in 0..20 {
        out[i] = a[i] ^ b[i];
    }
    out
}

fn derive_keys(shared: &[u8], info_hash: &[u8; 20], initiator: bool) -> ([u8; 20], [u8; 20]) {
    let key_a = sha1_bytes(b"keyA", &[shared, info_hash.as_slice()].concat());
    let key_b = sha1_bytes(b"keyB", &[shared, info_hash.as_slice()].concat());
    if initiator {
        (key_a, key_b)
    } else {
        (key_b, key_a)
    }
}

fn validate_peer_public(peer_pub: &BigUint) -> Result<(), String> {
    let modulus = prime()?;
    let upper = &modulus - BigUint::from(2u8);
    if peer_pub < &BigUint::from(2u8) || peer_pub > &upper {
        return Err("mse invalid DH public key".to_string());
    }
    Ok(())
}

fn dh_generate() -> Result<(BigUint, BigUint), String> {
    let mut priv_bytes = [0u8; 96];
    getrandom::fill(&mut priv_bytes)
        .map_err(|err| format!("mse random generation failed: {err}"))?;
    let modulus = prime()?;
    let range = &modulus - BigUint::from(3u8);
    let priv_key = (BigUint::from_bytes_be(&priv_bytes) % range) + BigUint::from(2u8);
    let pub_key = BigUint::from(2u8).modpow(&priv_key, &modulus);
    Ok((priv_key, pub_key))
}

fn to_fixed_bytes(value: &BigUint, len: usize) -> Vec<u8> {
    let mut bytes = value.to_bytes_be();
    if bytes.len() > len {
        bytes = bytes[bytes.len() - len..].to_vec();
    }
    if bytes.len() < len {
        let mut out = vec![0u8; len - bytes.len()];
        out.extend_from_slice(&bytes);
        return out;
    }
    bytes
}

fn prime() -> Result<BigUint, String> {
    let hex = "\
FFFFFFFFFFFFFFFFC90FDAA22168C234C4C6628B80DC1CD1\
29024E088A67CC74020BBEA63B139B22514A08798E3404DD\
EF9519B3CD3A431B302B0A6DF25F14374FE1356D6D51C245\
E485B576625E7EC6F44C42E9A63A3620FFFFFFFFFFFFFFFF";
    BigUint::from_str_radix(hex, 16).map_err(|_| "invalid built-in MSE prime".to_string())
}

#[derive(Clone)]
struct Rc4 {
    s: [u8; 256],
    i: u8,
    j: u8,
}

impl Rc4 {
    fn new(key: &[u8]) -> Self {
        let mut s = [0u8; 256];
        for (i, slot) in s.iter_mut().enumerate() {
            *slot = i as u8;
        }
        let mut j = 0u8;
        for i in 0..256u16 {
            let idx = i as usize;
            j = j.wrapping_add(s[idx]).wrapping_add(key[idx % key.len()]);
            s.swap(idx, j as usize);
        }
        Self { s, i: 0, j: 0 }
    }

    fn apply(&mut self, data: &mut [u8]) {
        for byte in data {
            self.i = self.i.wrapping_add(1);
            self.j = self.j.wrapping_add(self.s[self.i as usize]);
            self.s.swap(self.i as usize, self.j as usize);
            let idx = self.s[self.i as usize].wrapping_add(self.s[self.j as usize]);
            let k = self.s[idx as usize];
            *byte ^= k;
        }
    }

    fn discard(&mut self, count: usize) {
        let mut buf = vec![0u8; count];
        self.apply(&mut buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    use crate::peer_stream::PeerStream;

    /// Makes selected reads deterministic so a test can guarantee that the
    /// MSE scanner over-reads negotiation and post-negotiation bytes together.
    struct ExactReadTcp {
        inner: TcpStream,
        read_sizes: VecDeque<usize>,
    }

    impl ExactReadTcp {
        fn new(inner: TcpStream, read_sizes: impl IntoIterator<Item = usize>) -> Self {
            Self {
                inner,
                read_sizes: read_sizes.into_iter().collect(),
            }
        }
    }

    impl Read for ExactReadTcp {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let Some(read_len) = self.read_sizes.pop_front() else {
                return self.inner.read(buf);
            };
            if read_len > buf.len() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "scripted read is larger than destination",
                ));
            }
            self.inner.read_exact(&mut buf[..read_len])?;
            Ok(read_len)
        }
    }

    impl Write for ExactReadTcp {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.inner.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.inner.flush()
        }
    }

    #[test]
    fn xor_and_key_derivation_are_consistent() {
        let a = [0xAAu8; 20];
        let b = [0x0Fu8; 20];
        let x = xor_hash(&a, &b);
        assert_eq!(x, [0xA5u8; 20]);

        let shared = [3u8; 96];
        let info_hash = [9u8; 20];
        let (i_enc, i_dec) = derive_keys(&shared, &info_hash, true);
        let (r_enc, r_dec) = derive_keys(&shared, &info_hash, false);
        assert_eq!(i_enc, r_dec);
        assert_eq!(i_dec, r_enc);
    }

    #[test]
    fn to_fixed_bytes_pads_and_truncates() {
        let small = BigUint::from(0x1234u32);
        let padded = to_fixed_bytes(&small, 4);
        assert_eq!(padded, vec![0x00, 0x00, 0x12, 0x34]);

        let large = BigUint::from_str_radix("1122334455", 16).unwrap();
        let truncated = to_fixed_bytes(&large, 3);
        assert_eq!(truncated, vec![0x33, 0x44, 0x55]);
    }

    #[test]
    fn synchronization_pattern_cannot_bypass_padding_limit() {
        let pattern = b"marker";
        let mut data = vec![0u8; 520];
        data[512..518].copy_from_slice(pattern);
        assert_eq!(find_sync_pattern(&data, pattern, 0, 512), Some(512));

        let mut too_late = vec![0u8; 521];
        too_late[513..519].copy_from_slice(pattern);
        assert_eq!(find_sync_pattern(&too_late, pattern, 0, 512), None);
    }

    #[test]
    fn cipher_state_roundtrip() {
        let mut cipher = CipherState::new(b"key", b"key");
        let mut data = b"hello world".to_vec();
        let original = data.clone();
        cipher.encrypt(&mut data);
        assert_ne!(data, original);
        cipher.decrypt(&mut data);
        assert_eq!(data, original);
    }

    #[test]
    fn find_info_hash_matches_req2_digest() {
        let target = [7u8; 20];
        let other = [8u8; 20];
        let req2_target = sha1_bytes(b"req2", &target);
        let found = find_info_hash(&[other, target], &req2_target);
        assert_eq!(found, Some(target));
    }

    #[test]
    fn mse_initiate_accept_roundtrip_over_tcp() {
        let info_hash = [5u8; 20];
        let initial_payload = b"bt-handshake";
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut first = [0u8; 1];
            stream.read_exact(&mut first).unwrap();
            let (mode, mut cipher, matched_hash, ia, buffered) =
                accept(&mut stream, &[info_hash], first[0], false).unwrap();
            assert!(matches!(mode, CryptoMode::Rc4));
            assert_eq!(matched_hash, info_hash);
            assert_eq!(ia, initial_payload);
            assert!(buffered.is_empty());

            let mut outbound = b"pong".to_vec();
            if let Some(c) = cipher.as_mut() {
                c.encrypt(&mut outbound);
            }
            stream.write_all(&outbound).unwrap();

            let mut inbound = [0u8; 4];
            stream.read_exact(&mut inbound).unwrap();
            if let Some(c) = cipher.as_mut() {
                c.decrypt(&mut inbound);
            }
            assert_eq!(&inbound, b"ping");
        });

        let mut client = PeerStream::tcp(TcpStream::connect(addr).unwrap());
        let (mode, cipher, buffered) =
            initiate(&mut client, info_hash, false, initial_payload).unwrap();
        assert!(matches!(mode, CryptoMode::Rc4));
        if let Some(cipher) = cipher {
            client.enable_encryption(cipher);
        }
        client.prepend_read_buffer(buffered);

        let mut inbound = [0u8; 4];
        client.read_exact(&mut inbound).unwrap();
        assert_eq!(&inbound, b"pong");

        client.write_all(b"ping").unwrap();
        server.join().unwrap();
    }

    #[test]
    fn initiate_preserves_plaintext_overread_after_pad_d() {
        let info_hash = [0x31u8; 20];
        let initial_payload = b"initiator-ia";
        let post_pad = b"plaintext-peer-handshake";
        let pad_d = [0xA1, 0xB2, 0xC3];
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let responder = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();

            let mut ya = [0u8; 96];
            stream.read_exact(&mut ya).unwrap();
            let ya = BigUint::from_bytes_be(&ya);
            validate_peer_public(&ya).unwrap();

            let (private, public) = dh_generate().unwrap();
            stream.write_all(&to_fixed_bytes(&public, 96)).unwrap();
            let shared = ya.modpow(&private, &prime().unwrap());
            let shared = to_fixed_bytes(&shared, 96);

            let mut req1 = [0u8; 20];
            let mut obfuscated_req2 = [0u8; 20];
            stream.read_exact(&mut req1).unwrap();
            stream.read_exact(&mut obfuscated_req2).unwrap();
            assert_eq!(req1, sha1_bytes(b"req1", &shared));
            assert_eq!(
                obfuscated_req2,
                xor_hash(
                    &sha1_bytes(b"req2", &info_hash),
                    &sha1_bytes(b"req3", &shared),
                )
            );

            let (initiator_key, responder_key) = derive_keys(&shared, &info_hash, true);
            let mut dec = Rc4::new(&initiator_key);
            dec.discard(1024);

            let mut header = [0u8; 14];
            stream.read_exact(&mut header).unwrap();
            dec.apply(&mut header);
            assert_eq!(&header[..8], &[0u8; 8]);
            let offered = u32::from_be_bytes(header[8..12].try_into().unwrap());
            assert_ne!(offered & 0x01, 0);
            let pad_c_len = u16::from_be_bytes(header[12..14].try_into().unwrap()) as usize;
            let mut pad_c = vec![0u8; pad_c_len];
            stream.read_exact(&mut pad_c).unwrap();
            dec.apply(&mut pad_c);

            let mut ia_len = [0u8; 2];
            stream.read_exact(&mut ia_len).unwrap();
            dec.apply(&mut ia_len);
            let mut ia = vec![0u8; u16::from_be_bytes(ia_len) as usize];
            stream.read_exact(&mut ia).unwrap();
            dec.apply(&mut ia);
            assert_eq!(ia, initial_payload);

            let mut response = Vec::new();
            response.extend_from_slice(&[0u8; 8]);
            response.extend_from_slice(&1u32.to_be_bytes());
            response.extend_from_slice(&(pad_d.len() as u16).to_be_bytes());
            response.extend_from_slice(&pad_d);
            let mut enc = Rc4::new(&responder_key);
            enc.discard(1024);
            enc.apply(&mut response);
            response.extend_from_slice(post_pad);
            stream.write_all(&response).unwrap();
        });

        let response_len = 8 + 4 + 2 + pad_d.len() + post_pad.len();
        let stream = TcpStream::connect(addr).unwrap();
        let mut stream = ExactReadTcp::new(stream, [96, response_len]);
        let (mode, cipher, buffered) =
            initiate(&mut stream, info_hash, true, initial_payload).unwrap();
        assert!(matches!(mode, CryptoMode::Plaintext));
        assert!(cipher.is_none());
        assert_eq!(buffered, post_pad);
        responder.join().unwrap();
    }

    #[test]
    fn accept_preserves_plaintext_overread_after_initial_payload() {
        let info_hash = [0x52u8; 20];
        let initial_payload = b"plaintext-ia";
        let post_ia = b"plaintext-message-after-ia";
        let pad_c = [0x19, 0x28, 0x37, 0x46];
        let transcript_len =
            20 + 20 + 8 + 4 + 2 + pad_c.len() + 2 + initial_payload.len() + post_ia.len();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let responder = thread::spawn(move || {
            let (mut raw_stream, _) = listener.accept().unwrap();
            let mut first = [0u8; 1];
            raw_stream.read_exact(&mut first).unwrap();
            let mut stream = ExactReadTcp::new(raw_stream, [95, transcript_len]);
            let (mode, cipher, matched_hash, ia, buffered) =
                accept(&mut stream, &[info_hash], first[0], true).unwrap();
            assert!(matches!(mode, CryptoMode::Plaintext));
            assert!(cipher.is_none());
            assert_eq!(matched_hash, info_hash);
            assert_eq!(ia, initial_payload);
            assert_eq!(buffered, post_ia);
        });

        let mut stream = TcpStream::connect(addr).unwrap();
        let (private, public) = dh_generate().unwrap();
        stream.write_all(&to_fixed_bytes(&public, 96)).unwrap();

        let mut yb = [0u8; 96];
        stream.read_exact(&mut yb).unwrap();
        let yb = BigUint::from_bytes_be(&yb);
        validate_peer_public(&yb).unwrap();
        let shared = yb.modpow(&private, &prime().unwrap());
        let shared = to_fixed_bytes(&shared, 96);
        let (initiator_key, responder_key) = derive_keys(&shared, &info_hash, true);

        let mut encrypted = Vec::new();
        encrypted.extend_from_slice(&[0u8; 8]);
        encrypted.extend_from_slice(&1u32.to_be_bytes());
        encrypted.extend_from_slice(&(pad_c.len() as u16).to_be_bytes());
        encrypted.extend_from_slice(&pad_c);
        encrypted.extend_from_slice(&(initial_payload.len() as u16).to_be_bytes());
        encrypted.extend_from_slice(initial_payload);
        let mut enc = Rc4::new(&initiator_key);
        enc.discard(1024);
        enc.apply(&mut encrypted);

        let mut transcript = Vec::with_capacity(transcript_len);
        transcript.extend_from_slice(&sha1_bytes(b"req1", &shared));
        transcript.extend_from_slice(&xor_hash(
            &sha1_bytes(b"req2", &info_hash),
            &sha1_bytes(b"req3", &shared),
        ));
        transcript.extend_from_slice(&encrypted);
        transcript.extend_from_slice(post_ia);
        assert_eq!(transcript.len(), transcript_len);
        stream.write_all(&transcript).unwrap();

        let mut response = [0u8; 14];
        stream.read_exact(&mut response).unwrap();
        let mut dec = Rc4::new(&responder_key);
        dec.discard(1024);
        dec.apply(&mut response);
        assert_eq!(&response[..8], &[0u8; 8]);
        assert_eq!(u32::from_be_bytes(response[8..12].try_into().unwrap()), 1);
        assert_eq!(u16::from_be_bytes(response[12..14].try_into().unwrap()), 0);

        responder.join().unwrap();
    }
}
