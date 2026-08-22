#![no_main]
#![allow(dead_code)]

use libfuzzer_sys::fuzz_target;

mod bencode {
    include!("../../src/bencode.rs");
}

mod sha1 {
    include!("../../src/sha1.rs");
}

mod sha256 {
    include!("../../src/sha256.rs");
}

mod torrent {
    include!("../../src/torrent.rs");
}

fuzz_target!(|data: &[u8]| {
    let _ = torrent::parse_torrent(data);
});
