#![no_main]
#![allow(dead_code)]

use libfuzzer_sys::fuzz_target;

mod bencode {
    include!("../../src/bencode.rs");
}

fuzz_target!(|data: &[u8]| {
    if let Ok(value) = bencode::parse(data) {
        let encoded = bencode::encode(&value);
        assert_eq!(bencode::parse(&encoded), Ok(value));
    }
    if !data.is_empty() {
        let _ = bencode::parse_value(data, 0);
        let _ = bencode::parse_value(data, data.len() / 2);
        let _ = bencode::parse_value(data, data.len() - 1);
    }
});
