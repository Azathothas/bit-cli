#![no_main]
#![allow(dead_code)]

use libfuzzer_sys::fuzz_target;
use std::ffi::OsString;

fn clean_segment(bytes: &[u8]) -> Result<OsString, ()> {
    if bytes.is_empty() {
        return Err(());
    }
    if bytes == b"." || bytes == b".." {
        return Err(());
    }
    if bytes.iter().any(|b| *b == 0 || *b == b'/' || *b == b'\\') {
        return Err(());
    }
    if bytes.iter().any(|b| *b < 0x20) {
        return Err(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(OsStringExt::from_vec(bytes.to_vec()))
    }
    #[cfg(not(unix))]
    {
        let invalid = [b':', b'*', b'?', b'"', b'<', b'>', b'|'];
        if bytes.iter().any(|byte| invalid.contains(byte))
            || matches!(bytes.last(), Some(b'.' | b' '))
        {
            return Err(());
        }
        let name = String::from_utf8(bytes.to_vec()).map_err(|_| ())?;
        let upper = name.trim_end_matches([' ', '.']).to_ascii_uppercase();
        let stem = upper.split('.').next().unwrap_or(&upper).trim_end();
        if matches!(
            stem,
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        ) {
            return Err(());
        }
        Ok(OsString::from(name))
    }
}

fuzz_target!(|data: &[u8]| {
    // Fuzz clean_segment with raw bytes.
    let _ = clean_segment(data);

    // Also fuzz with segments derived from splitting the input.
    for segment in data.split(|b| *b == b'/') {
        let _ = clean_segment(segment);
    }
});
