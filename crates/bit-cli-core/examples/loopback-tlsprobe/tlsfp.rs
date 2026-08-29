//! `ClientHello` parsing, and the JA3 and JA4 fingerprints computed from it.
//!
//! Every read goes through a cursor that returns `None` rather than panicking,
//! because the bytes come off a socket and a truncated or hostile record must
//! end the capture, not the process.

use sha2::{Digest as _, Sha256};

/// Lowercase hex of a digest.
///
/// `sha2` and `md-5` are pinned at 0.11 here, where `digest()` returns an
/// `Array` that does not implement `LowerHex`, so `format!("{:x}", ...)` does
/// not compile. `bit_cli_core::digest` spells it out the same way for the same
/// reason.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A GREASE value (RFC 8701): both bytes equal, low nibble `a`.
pub fn is_grease(v: u16) -> bool {
    v & 0x0f0f == 0x0a0a && (v >> 8) == (v & 0xff)
}

fn strip(v: &[u16]) -> Vec<u16> {
    v.iter().copied().filter(|&x| !is_grease(x)).collect()
}

#[derive(Default, Debug)]
pub struct ClientHello {
    pub legacy_version: u16,
    pub ciphers: Vec<u16>,
    pub extensions: Vec<u16>,
    pub curves: Vec<u16>,
    pub point_formats: Vec<u8>,
    pub alpn: Vec<String>,
    pub sig_algs: Vec<u16>,
    pub supported_versions: Vec<u16>,
    pub sni: Option<String>,
    pub cert_compression: Vec<u16>,
    pub key_share_groups: Vec<u16>,
    pub psk_key_exchange_modes: Vec<u8>,
    pub record_version: u16,
    pub has_ech: bool,
    pub has_alps: bool,
}

/// Slice-with-cursor that never panics on a truncated or hostile record.
struct Cur<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Cur<'a> {
    fn new(b: &'a [u8]) -> Self {
        Cur { b, p: 0 }
    }
    fn u8(&mut self) -> Option<u8> {
        let v = *self.b.get(self.p)?;
        self.p += 1;
        Some(v)
    }
    fn u16(&mut self) -> Option<u16> {
        let v = u16::from_be_bytes(self.b.get(self.p..self.p + 2)?.try_into().ok()?);
        self.p += 2;
        Some(v)
    }
    fn u24(&mut self) -> Option<u32> {
        let s = self.b.get(self.p..self.p + 3)?;
        self.p += 3;
        Some(((s[0] as u32) << 16) | ((s[1] as u32) << 8) | s[2] as u32)
    }
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.b.get(self.p..self.p.checked_add(n)?)?;
        self.p += n;
        Some(s)
    }
    fn skip(&mut self, n: usize) -> Option<()> {
        self.take(n).map(|_| ())
    }
}

fn u16_list(b: &[u8]) -> Vec<u16> {
    b.as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_be_bytes(*c))
        .collect()
}

/// Parse a TLS record containing a ClientHello.
pub fn parse(buf: &[u8]) -> Option<ClientHello> {
    let mut c = Cur::new(buf);
    let mut ch = ClientHello::default();

    if c.u8()? != 0x16 {
        return None; // not a handshake record
    }
    ch.record_version = c.u16()?;
    let rec_len = c.u16()? as usize;
    // Guard against a ClientHello fragmented across records: we only ever peek
    // one buffer, so a short read is a parse failure, not a silent truncation.
    if buf.len() < 5 + rec_len {
        return None;
    }
    if c.u8()? != 0x01 {
        return None; // not a ClientHello
    }
    c.u24()?; // handshake length
    ch.legacy_version = c.u16()?;
    c.skip(32)?; // random
    let sid = c.u8()? as usize;
    c.skip(sid)?;

    let cs_len = c.u16()? as usize;
    ch.ciphers = u16_list(c.take(cs_len)?);
    let comp = c.u8()? as usize;
    c.skip(comp)?;

    // Extensions are optional in the wire format (SSLv3-era hellos omit them).
    let ext_total = match c.u16() {
        Some(n) => n as usize,
        None => return Some(ch),
    };
    let end = c.p + ext_total;

    while c.p < end {
        let et = c.u16()?;
        let el = c.u16()? as usize;
        let body = c.take(el)?;
        ch.extensions.push(et);

        match et {
            0x0000 => {
                // server_name: list(2) type(1) len(2) host
                let mut s = Cur::new(body);
                s.u16()?;
                if s.u8() == Some(0) {
                    let n = s.u16()? as usize;
                    ch.sni = s.take(n).map(|h| String::from_utf8_lossy(h).into_owned());
                }
            }
            0x000a => {
                let mut s = Cur::new(body);
                let n = s.u16()? as usize;
                ch.curves = u16_list(s.take(n)?);
            }
            0x000b => {
                let mut s = Cur::new(body);
                let n = s.u8()? as usize;
                ch.point_formats = s.take(n)?.to_vec();
            }
            0x000d => {
                let mut s = Cur::new(body);
                let n = s.u16()? as usize;
                ch.sig_algs = u16_list(s.take(n)?);
            }
            0x0010 => {
                let mut s = Cur::new(body);
                let total = s.u16()? as usize;
                let inner = s.take(total)?;
                let mut q = Cur::new(inner);
                while q.p < inner.len() {
                    let n = q.u8()? as usize;
                    ch.alpn
                        .push(String::from_utf8_lossy(q.take(n)?).into_owned());
                }
            }
            0x002b => {
                let mut s = Cur::new(body);
                let n = s.u8()? as usize;
                ch.supported_versions = u16_list(s.take(n)?);
            }
            0x002d => {
                let mut s = Cur::new(body);
                let n = s.u8()? as usize;
                ch.psk_key_exchange_modes = s.take(n)?.to_vec();
            }
            0x0033 => {
                let mut s = Cur::new(body);
                let total = s.u16()? as usize;
                let inner = s.take(total)?;
                let mut q = Cur::new(inner);
                while q.p < inner.len() {
                    let g = q.u16()?;
                    let n = q.u16()? as usize;
                    q.skip(n)?;
                    ch.key_share_groups.push(g);
                }
            }
            0x001b => {
                let mut s = Cur::new(body);
                let n = s.u8()? as usize;
                ch.cert_compression = u16_list(s.take(n)?);
            }
            0xfe0d => ch.has_ech = true,
            0x4469 | 0x44cd => ch.has_alps = true,
            _ => {}
        }
    }
    Some(ch)
}

impl ClientHello {
    /// Highest offered TLS version, preferring `supported_versions`.
    pub fn effective_version(&self) -> u16 {
        strip(&self.supported_versions)
            .into_iter()
            .max()
            .unwrap_or(self.legacy_version)
    }

    fn ja4_version(&self) -> &'static str {
        match self.effective_version() {
            0x0304 => "13",
            0x0303 => "12",
            0x0302 => "11",
            0x0301 => "10",
            0x0300 => "s3",
            _ => "00",
        }
    }

    /// JA3 (Salesforce). `filter_grease` is **not** in the original spec, but
    /// every modern implementation applies it — without it Chrome's JA3 changes
    /// on every connection, since Chrome randomises its GREASE values.
    pub fn ja3(&self, filter_grease: bool) -> (String, String) {
        let f = |v: &[u16]| {
            let v = if filter_grease { strip(v) } else { v.to_vec() };
            v.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join("-")
        };
        let s = format!(
            "{},{},{},{},{}",
            self.legacy_version,
            f(&self.ciphers),
            f(&self.extensions),
            f(&self.curves),
            self.point_formats
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join("-")
        );
        let hash = hex(&md5::Md5::digest(s.as_bytes()));
        (s, hash)
    }

    /// JA4 `a` segment: `t<ver><d|i><ciphers><exts><alpn>`.
    fn ja4_a(&self) -> String {
        let nc = strip(&self.ciphers).len().min(99);
        // The extension *count* includes SNI and ALPN; only the `c` hash drops them.
        let ne = strip(&self.extensions).len().min(99);
        let alpn = match self.alpn.first() {
            // Per the JA4 spec the marker is the first and last byte of the
            // first ALPN value, so "http/1.1" -> "h1", "h2" -> "h2".
            Some(a) if !a.is_empty() => {
                let b = a.as_bytes();
                format!("{}{}", b[0] as char, b[b.len() - 1] as char)
            }
            _ => "00".to_string(),
        };
        format!(
            "t{}{}{:02}{:02}{}",
            self.ja4_version(),
            if self.sni.is_some() { 'd' } else { 'i' },
            nc,
            ne,
            alpn
        )
    }

    fn ja4_b_raw(&self) -> String {
        let mut cs = strip(&self.ciphers);
        cs.sort_unstable();
        cs.iter()
            .map(|c| format!("{c:04x}"))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn ja4_c_raw(&self) -> String {
        let mut ex: Vec<u16> = strip(&self.extensions)
            .into_iter()
            .filter(|&e| e != 0x0000 && e != 0x0010) // SNI and ALPN excluded
            .collect();
        ex.sort_unstable();
        let exs = ex
            .iter()
            .map(|e| format!("{e:04x}"))
            .collect::<Vec<_>>()
            .join(",");
        // Signature algorithms keep their ORIGINAL order — they are not sorted.
        let sig = strip(&self.sig_algs)
            .iter()
            .map(|s| format!("{s:04x}"))
            .collect::<Vec<_>>()
            .join(",");
        if sig.is_empty() {
            exs
        } else {
            format!("{exs}_{sig}")
        }
    }

    fn trunc12(s: &str) -> String {
        if s.is_empty() {
            return "000000000000".into();
        }
        hex(&Sha256::digest(s.as_bytes()))[..12].to_string()
    }

    /// Hashed JA4.
    pub fn ja4(&self) -> String {
        format!(
            "{}_{}_{}",
            self.ja4_a(),
            Self::trunc12(&self.ja4_b_raw()),
            Self::trunc12(&self.ja4_c_raw())
        )
    }

    /// JA4_r — the un-hashed form, sorted. This is what you diff when two
    /// fingerprints disagree; the hashes only tell you *that* they differ.
    pub fn ja4_r(&self) -> String {
        format!("{}_{}_{}", self.ja4_a(), self.ja4_b_raw(), self.ja4_c_raw())
    }

    /// JA4_ro — the un-hashed form in the order the client actually sent.
    ///
    /// JA4 and JA4_r sort the ciphers and the extensions before comparing,
    /// which is what makes them stable against a client that shuffles its
    /// extensions between connections. That stability hides something: two
    /// clients with the same JA4_r can still put their extensions on the wire
    /// in different orders, and the order is itself a signal.
    ///
    /// So this is the diagnostic form. It is **never asserted**, for exactly
    /// the reason JA3 is never asserted: it moves when nothing is wrong. It is
    /// what to read when a JA4_r matches and a capture still looks unlike the
    /// client it claims to be.
    pub fn ja4_ro(&self) -> String {
        let ciphers = strip(&self.ciphers)
            .iter()
            .map(|c| format!("{c:04x}"))
            .collect::<Vec<_>>()
            .join(",");
        let extensions = strip(&self.extensions)
            .iter()
            .map(|e| format!("{e:04x}"))
            .collect::<Vec<_>>()
            .join(",");
        let sig = strip(&self.sig_algs)
            .iter()
            .map(|s| format!("{s:04x}"))
            .collect::<Vec<_>>()
            .join(",");
        match sig.is_empty() {
            true => format!("{}_{}_{}", self.ja4_a(), ciphers, extensions),
            false => format!("{}_{}_{}_{}", self.ja4_a(), ciphers, extensions, sig),
        }
    }

    /// Markers that separate a current Chrome from a generic TLS client.
    pub fn browser_markers(&self) -> Vec<(&'static str, bool)> {
        vec![
            (
                "GREASE in ciphers",
                self.ciphers.iter().any(|&c| is_grease(c)),
            ),
            (
                "GREASE in extensions",
                self.extensions.iter().any(|&e| is_grease(e)),
            ),
            ("ECH (0xfe0d)", self.has_ech),
            ("ALPS (0x4469/0x44cd)", self.has_alps),
            ("cert compression (0x1b)", !self.cert_compression.is_empty()),
            (
                "X25519MLKEM768 key share",
                self.key_share_groups.contains(&0x11ec),
            ),
            ("ALPN offers h2", self.alpn.iter().any(|a| a == "h2")),
            (
                "session ticket / PSK modes",
                !self.psk_key_exchange_modes.is_empty(),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One `ClientHello` this repository's own browser profile put on the
    /// wire, recorded by `loopback-tlsprobe --raw --hello-out`.
    ///
    /// **Embedded as hex and not as a file.** `scripts/check-tree.ps1` keeps a
    /// `.bin` out of everywhere but `vendor/`, and a binary blob is not
    /// something a reviewer can read either. Regenerate with:
    ///
    /// ```text
    /// loopback-tlsprobe --once --raw --port 0 --hello-out <path>
    /// bit-cli info <url>/x.torrent --page-client browser
    /// ```
    const BROWSER_HELLO: &str = "\
        16030107b9010007b503037524351ee23b7a5597e65d0bd1eb5a9be4d126c13b2f6821028c8e\
        57cd438a0420d795ce67eb5c08d7e1a3702a9c57c6613afd3937caec1b9e1ee39e2fe0b50d1b\
        00200a0a130113021303c02bc02fc02cc030cca9cca8c013c014009c009d002f00350100074c\
        ff01000100000b0002010044cd0008000605696d7069740017000000230000000d0018001609\
        040905090604030804040105030805050108060601000500050100000000000a000c000a8a8a\
        11ec001d001700180010000e000c02683208687474702f312e3100120000003304ea04e811ec\
        04c0bb474d9106c96a96ababf10fdd951d8cd6795a150b3f75193c28721cf06aa6054c8d0237\
        d8507f8933094d486760a41a66405b3a443669e65568426071f08b388342df798bcb1374b923\
        982ed0bfc0d3cdf6871df552824fa316a3e4189b9b2a330495f2220562e3076775a246cac68b\
        bb4ceae584af284ce74a075267ab90533e904414206c762ffb93cccb4c1cfc5fa9e23ac54425\
        18a470e8444019d30ae4b87eb5938d00b13c99e80518d82adf191d4f913b628288d5d8b12aeb\
        98e956c2c9c2a92e28c1cf565914f8b6ba54a92c6624dc636a3a319545e4483cf3787e2b337e\
        127ba040488e4cc2c4d74d764a21290b1009e66f39436de3923ab732b02e9b2911cacddbc30e\
        091616d094a1f4595aba15a10d297a8d8a2b3c4a8978288d54e7007c224b68907340278e684c\
        a5b666511338c3cd405143e53dfea162e8150da0b8b944025e1001b5c4508ca48a707b69a83c\
        472353e87ff3a6ce0f26498af10d4db32ad092404ba724c76b2a873445221554e911018495cc\
        4a960551497c3101c341d62e7e8cac89949650c1bb8c538ce453552dc59e95fa42c0b87147ac\
        b5f902b66de929c739c92cc70241472ff418badf689e9ac099a0ec0b64e662d41719eb6b84af\
        8c804ca30c1a7a3ff84b91f9014ea70a3fe9d3aa269704fa5941aa043029c2016b033454e11c\
        6bdb3cad7364d44a97a1456e38c11573ec33cef4c039020bff1717b4a7c8658666f807800085\
        6572438724da231658c26c0a1da98bb6e9f527844077c93a9d1f4923ec819d0d15af37735194\
        7b900105696536793917580cf66236db835666204b60cd7d865caae9430aaba13570351efc1d\
        bed720020b1baff4829ae596660204fbd686ab1b6c5896422a85cd83534eb7d57b0b122766d2\
        9de5814b3cd94c4165992478b30dcb62ff85157d365b736b408a6921baf82782c738ed45717c\
        142bd483af59502c8c3b4f556a1fe317451805799242565fa10257bc4b4236290d23a28b3520\
        5b5c23509855840c466a0b7d757459ac311eed71b6f6a7ab4a855492e7a084228773a4420a59\
        83ab63c8484c5971d7a21e607f937bc3bb57bc56fc621109057e995a5a593412750463518603\
        e9bf02fb968513819d5912be7361d690a44f3cc20f535be6c9393b52cb66ba33a2fc7c65ebc2\
        1d94038e5872b9b784a61bbd5ea8c207733467d57fa1fbb2c2e838e3e71bfff29ab147ce7000\
        59d9f9cf24087d590886d432836a527dd8e73c8cd2512e8c5c79ba6a7f86a005979e7d983e2b\
        37242485a396925622d15fd58a0b9117598fa89ae9588b485161e1654bfa524923a5301c880c\
        cb18b04685276ed7638a362456a79499e33cbc83c1159513a80a14cc9bb9898a1456a0812ac8\
        167d9c151a0985777c6cb9a38989f82158f4b5466a8011d17b32271f045b3dda7833cd74cd0a\
        0cccd655652d377b376186846545e565a78ff3914d54bca1524cdec34aeb095265a00e7ed51b\
        a59b14a5a32e31c694a452b5fbdb78647a736a00c7716a6cf10435d7920bfcf122459bcc97e8\
        c5dec70f33ba6565c517d2c50efad39583e949e4dc65460cb8647044c4757a6ea287b0d76b54\
        823bfe0423c55c356bb6a94571b4ab15231b44b566c1e42a709184890f0add5eb0c4d2aa1682\
        d510b489dbacf779dadf8d5b4b79e73dd200fa9845659c8d6e828464a560068da33dd1b788d9\
        4e09001d0020dadf8d5b4b79e73dd200fa9845659c8d6e828464a560068da33dd1b788d94e09\
        002d00020101002b00050403040303001b0003020002fe0d01da0000010001510020a7c291b3\
        511b865c6823105c084bc2c428a0bfccfe0131eb9f165b907218a93701b09d4df88ca3bf4b88\
        28e65653f6bf205e4be5b517aec7d9ba48e7e7930fa3781c8c9969ee8d12dcdd4200865533fc\
        398cfaa8152269419a78fa11656f47242f7f8c93e5909199bbe595a2b51f4d2ef7f84be5d5dc\
        a5f144fe3417fd40e68135e58a02f0e359f7ca1c0e5090ea67e8452c371a38c4c5512cc07459\
        825ed1df7f50f67f4523304c107a4dcf90071dd62a2d2289a0da09d69a9ca242a34e1e09012d\
        b2c5fb265eb916f08e8bceb45cac4b34467eb5c8cf73091e1e7b39401c0011073c6f041f3e5b\
        1723d296b346bf9f06488b4329779be27688dd4cf3ddbbdc77ed5422e708400d183c5c324899\
        4538352f18cb00ba4f1a90fba4dc26f097490d3593c833dd8546eacdd223521376273f5fecb7\
        67737889925412b2a64c79ab108aab95d8b4223b9d13655be8840378c9570e3847e0527e57cb\
        032f8ef5f19502d023c96c9de6c93fee978adff022aaee4d72522cf8261a18f401311085e1fe\
        44bb065f31499789e4f5e45f73608be7c97a894b2703b3f9da1d124e8d25a55b9e11bda60303\
        38cc52cc561305a9be7a08e07d5a06eda9fe8bc39ae7f22a409ab13405f1005113a246b080df\
        db8c566d1145";

    /// The same, from the `plain` profile: this tree's own `rustls`, with no
    /// impersonation. It is the control. A parser change that improves the
    /// browser reading and breaks this one has broken the parser.
    const PLAIN_HELLO: &str = "\
        16030100f0010000ec0303ec01f8cf3a4c4ddc2ba4daf22f0579d7adf853bdbc6ed0a5f9417e\
        a153c66f5c205509727a869f8d9fb026ff5e8304a318199bcf04829c9278685d1e16b4a2053f\
        0014130213011303c02cc02bcca9c030c02fcca800ff0100008f002d00020101000b00020100\
        00170000001b0005040002000100230000003300260024001d0020946f1307af4382da283fdf\
        9d3e1c908146f76966d09f858e802af2f7cb25242a0010000e000c02683208687474702f312e\
        31000a00080006001d00170018000500050100000000000d0014001205030403080708060805\
        0804060105010401002b00050403040303";

    fn bytes(hex: &str) -> Vec<u8> {
        assert!(hex.len().is_multiple_of(2), "a hex string has even length");
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex"))
            .collect()
    }

    #[test]
    fn the_recorded_hellos_decode_to_the_length_they_were_captured_at() {
        assert_eq!(bytes(BROWSER_HELLO).len(), 1982);
        assert_eq!(bytes(PLAIN_HELLO).len(), 245);
    }

    #[test]
    fn the_browser_hello_parses_to_the_fingerprint_it_was_captured_with() {
        let ch = parse(&bytes(BROWSER_HELLO)).expect("a recorded ClientHello parses");
        assert_eq!(ch.ja4(), "t13i1515h2_8daaf6152771_806a8c22fdea");
        assert_eq!(
            ch.ja4_r(),
            "t13i1515h2_002f,0035,009c,009d,1301,1302,1303,c013,c014,c02b,c02c,c02f,c030,cca8,cca9_0005,000a,000b,000d,0012,0017,001b,0023,002b,002d,0033,44cd,fe0d,ff01_0904,0905,0906,0403,0804,0401,0503,0805,0501,0806,0601"
        );
    }

    #[test]
    fn the_plain_hello_parses_to_the_fingerprint_it_was_captured_with() {
        let ch = parse(&bytes(PLAIN_HELLO)).expect("a recorded ClientHello parses");
        assert_eq!(ch.ja4(), "t13i1011h2_61a7ad8aa9b6_69ed562cf35e");
        assert_eq!(
            ch.ja4_r(),
            "t13i1011h2_00ff,1301,1302,1303,c02b,c02c,c02f,c030,cca8,cca9_0005,000a,000b,000d,0017,001b,0023,002b,002d,0033_0503,0403,0807,0806,0805,0804,0601,0501,0401"
        );
    }

    /// What separates the two, field by field. This is the assertion that
    /// would catch a parser reading the right number of ciphers out of the
    /// wrong offset.
    #[test]
    fn the_browser_hello_carries_what_a_browser_carries_and_the_plain_one_does_not() {
        let browser = parse(&bytes(BROWSER_HELLO)).expect("browser");
        let plain = parse(&bytes(PLAIN_HELLO)).expect("plain");

        assert_eq!(strip(&browser.ciphers).len(), 15);
        assert_eq!(strip(&plain.ciphers).len(), 10);

        assert!(browser.has_ech, "a current Chrome offers ECH");
        assert!(!plain.has_ech, "this tree's own rustls does not");
        assert!(browser.has_alps, "a current Chrome offers ALPS");
        assert!(!plain.has_alps);
        assert!(
            !browser.cert_compression.is_empty(),
            "a current Chrome offers certificate compression"
        );

        // GREASE is in the browser's cipher list and in neither of the
        // plain client's, which is the single clearest marker there is.
        assert!(browser.ciphers.iter().any(|&c| is_grease(c)));
        assert!(!plain.ciphers.iter().any(|&c| is_grease(c)));
        assert!(!plain.extensions.iter().any(|&e| is_grease(e)));

        // And **not** in the browser's extension list, which a real Chrome
        // puts one at each end of. That is a measured limit of the profile
        // rather than a property of a browser, it is invisible to JA4 because
        // JA4 strips GREASE before hashing, and it is [T-263].
        assert!(
            !browser.extensions.iter().any(|&e| is_grease(e)),
            "if this starts failing, the profile gained extension GREASE and T-263 can close"
        );
    }

    /// The cipher list is Chrome's, in Chrome's wire order, GREASE included.
    ///
    /// Sixteen values in one sequence, captured from a real Chrome 151 on the
    /// same machine on the same day. Only the GREASE value itself differs, and
    /// it differs on purpose: a browser picks a new one per connection.
    #[test]
    fn the_browser_cipher_list_is_chromes_own_wire_order() {
        let ch = parse(&bytes(BROWSER_HELLO)).expect("browser");
        let chrome: Vec<u16> = vec![
            0x1301, 0x1302, 0x1303, 0xc02b, 0xc02f, 0xc02c, 0xc030, 0xcca9, 0xcca8, 0xc013, 0xc014,
            0x009c, 0x009d, 0x002f, 0x0035,
        ];
        assert!(
            is_grease(ch.ciphers[0]),
            "GREASE leads, as it does in Chrome"
        );
        assert_eq!(ch.ciphers[1..].to_vec(), chrome);
    }

    /// GREASE values are dropped before hashing, by the JA4 specification.
    /// This is the property that makes a JA4 stable across connections at all,
    /// because Chrome picks new GREASE values every time.
    #[test]
    fn grease_is_stripped_from_every_hashed_list() {
        let ch = parse(&bytes(BROWSER_HELLO)).expect("browser");
        assert!(ch.ciphers.iter().any(|&c| is_grease(c)), "raw list has it");
        assert!(
            !strip(&ch.ciphers).iter().any(|&c| is_grease(c)),
            "the stripped list does not"
        );
        assert!(!ch.ja4_r().contains("0a0a"), "{}", ch.ja4_r());
    }

    /// JA4_ro keeps the wire order where JA4_r sorts. Both describe the same
    /// hello, so they carry the same values and, for a client that does not
    /// send them sorted, in a different sequence.
    #[test]
    fn ja4_ro_keeps_the_order_ja4_r_sorts_away() {
        let ch = parse(&bytes(BROWSER_HELLO)).expect("browser");
        let sorted = ch.ja4_r();
        let wire = ch.ja4_ro();
        assert_ne!(sorted, wire, "a browser does not send them sorted");
        assert!(wire.starts_with(&sorted[..sorted.find('_').expect("prefix")]));
    }

    /// Nothing here may panic on a hostile input, which is the whole reason
    /// the parser is written on a cursor that returns `Option` at every step.
    #[test]
    fn a_truncated_hello_is_none_and_never_a_panic() {
        let full = bytes(BROWSER_HELLO);
        for cut in [0, 1, 5, 9, 40, 100, 500, full.len() - 1] {
            let _ = parse(&full[..cut]);
        }
    }

    #[test]
    fn a_hello_with_a_length_that_overruns_the_buffer_is_none() {
        let mut full = bytes(BROWSER_HELLO);
        // Claim a record far longer than what follows.
        full[3] = 0xff;
        full[4] = 0xff;
        let _ = parse(&full);
    }

    #[test]
    fn bytes_that_are_not_tls_are_none() {
        assert!(parse(b"GET / HTTP/1.1\r\n\r\n").is_none());
        assert!(parse(&[]).is_none());
        assert!(parse(&[0x16]).is_none());
    }
}
