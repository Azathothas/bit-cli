// 
// 

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    superseedr::fuzzing::parse_torrent_file(bytes);
});
