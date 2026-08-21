//! Bencode, encoded canonically.
//!
//! The info hash is the SHA-1 of the `info` dictionary's encoded bytes, so
//! every byte of that encoding matters. Two properties are load-bearing:
//!
//! - **Dictionary keys are sorted by raw byte value on output.** BEP 3
//!   requires it, and it is what makes `bit-cli create` byte-reproducible
//!   across platforms. A [`BTreeMap`] gives it for free, which is why this
//!   module exists rather than reusing a `HashMap`-backed decoder.
//! - **The `info` dictionary's raw bytes are kept.** `bit-cli edit` rewrites
//!   fields outside `info` and re-emits the original `info` bytes verbatim, so
//!   the info hash cannot drift even if this encoder and whatever produced the
//!   torrent disagree about canonical form.
//!
//! Decoding is strict about the things that would let two different encodings
//! mean the same thing: no leading zeros, no `i-0e`, no trailing data.

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;

/// A bencode value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    /// A byte string. Bencode has no notion of text encoding.
    Bytes(Vec<u8>),
    /// An integer.
    Int(i64),
    /// A list.
    List(Vec<Value>),
    /// A dictionary, sorted by raw key bytes.
    Dict(BTreeMap<Vec<u8>, Value>),
}

/// Why decoding failed. Every variant names the byte offset, because a
/// truncated torrent is otherwise very hard to diagnose.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("unexpected end of input at byte {0}")]
    Eof(usize),
    #[error("unexpected byte {byte:?} at byte {at}, expected a bencode value")]
    Unexpected { at: usize, byte: char },
    #[error("integer at byte {0} is malformed")]
    BadInteger(usize),
    #[error("integer at byte {0} has a leading zero or is `-0`")]
    NonCanonicalInteger(usize),
    #[error("byte string length at byte {0} is malformed")]
    BadLength(usize),
    #[error("byte string at byte {at} claims {claimed} bytes but only {available} remain")]
    LengthOverrun {
        at: usize,
        claimed: u64,
        available: usize,
    },
    #[error("dictionary key at byte {0} is not a byte string")]
    NonStringKey(usize),
    #[error("dictionary at byte {0} has a duplicate key")]
    DuplicateKey(usize),
    #[error("{trailing} unexpected bytes after the value at byte {at}")]
    TrailingData { at: usize, trailing: usize },
}

/// Decode one value, requiring it to consume the whole input.
pub fn decode(input: &[u8]) -> Result<Value, Error> {
    let (value, rest) = decode_prefix(input)?;
    if rest != input.len() {
        return Err(Error::TrailingData {
            at: rest,
            trailing: input.len() - rest,
        });
    }
    Ok(value)
}

/// Decode one value, returning it and the offset just past it.
pub fn decode_prefix(input: &[u8]) -> Result<(Value, usize), Error> {
    let mut parser = Parser {
        input,
        pos: 0,
        info_span: None,
    };
    let value = parser.value()?;
    Ok((value, parser.pos))
}

/// Decode a torrent, also returning the byte span of the top-level `info`
/// dictionary's value.
///
/// The span is what the info hash is computed over. Recomputing it by
/// re-encoding the parsed `info` would be wrong for any torrent whose original
/// encoding was not canonical, and such torrents exist in the wild.
pub fn decode_with_info_span(input: &[u8]) -> Result<(Value, Option<Range<usize>>), Error> {
    let mut parser = Parser {
        input,
        pos: 0,
        info_span: None,
    };
    let value = parser.value()?;
    if parser.pos != input.len() {
        return Err(Error::TrailingData {
            at: parser.pos,
            trailing: input.len() - parser.pos,
        });
    }
    Ok((value, parser.info_span))
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    /// Byte span of the value under the top-level `info` key, recorded when
    /// the outermost dictionary is parsed.
    info_span: Option<Range<usize>>,
}

impl Parser<'_> {
    fn peek(&self) -> Result<u8, Error> {
        self.input
            .get(self.pos)
            .copied()
            .ok_or(Error::Eof(self.pos))
    }

    fn value(&mut self) -> Result<Value, Error> {
        match self.peek()? {
            b'i' => self.integer(),
            b'l' => self.list(),
            b'd' => self.dict(),
            b'0'..=b'9' => self.bytes().map(Value::Bytes),
            byte => Err(Error::Unexpected {
                at: self.pos,
                byte: byte as char,
            }),
        }
    }

    fn integer(&mut self) -> Result<Value, Error> {
        let start = self.pos;
        self.pos += 1;
        let end = self.find(b'e').ok_or(Error::Eof(start))?;
        let digits = &self.input[self.pos..end];
        let text = std::str::from_utf8(digits).map_err(|_| Error::BadInteger(start))?;
        let value: i64 = text.parse().map_err(|_| Error::BadInteger(start))?;
        // `i03e` and `i-0e` would let one number have several encodings, which
        // would make the info hash ambiguous.
        let canonical = value.to_string();
        if canonical != text {
            return Err(Error::NonCanonicalInteger(start));
        }
        self.pos = end + 1;
        Ok(Value::Int(value))
    }

    fn bytes(&mut self) -> Result<Vec<u8>, Error> {
        let start = self.pos;
        let colon = self.find(b':').ok_or(Error::Eof(start))?;
        let digits = &self.input[start..colon];
        let text = std::str::from_utf8(digits).map_err(|_| Error::BadLength(start))?;
        let length: u64 = text.parse().map_err(|_| Error::BadLength(start))?;
        if length.to_string() != text {
            return Err(Error::BadLength(start));
        }
        let from = colon + 1;
        let available = self.input.len() - from;
        let wanted = usize::try_from(length).map_err(|_| Error::LengthOverrun {
            at: start,
            claimed: length,
            available,
        })?;
        if wanted > available {
            return Err(Error::LengthOverrun {
                at: start,
                claimed: length,
                available,
            });
        }
        self.pos = from + wanted;
        Ok(self.input[from..self.pos].to_vec())
    }

    fn list(&mut self) -> Result<Value, Error> {
        let start = self.pos;
        self.pos += 1;
        let mut items = Vec::new();
        loop {
            match self.input.get(self.pos) {
                None => return Err(Error::Eof(start)),
                Some(b'e') => {
                    self.pos += 1;
                    return Ok(Value::List(items));
                }
                Some(_) => items.push(self.value()?),
            }
        }
    }

    fn dict(&mut self) -> Result<Value, Error> {
        let start = self.pos;
        let outermost = start == 0;
        self.pos += 1;
        let mut map = BTreeMap::new();
        loop {
            match self.input.get(self.pos) {
                None => return Err(Error::Eof(start)),
                Some(b'e') => {
                    self.pos += 1;
                    return Ok(Value::Dict(map));
                }
                Some(b'0'..=b'9') => {
                    let key_at = self.pos;
                    let key = self.bytes()?;
                    let value_start = self.pos;
                    let value = self.value()?;
                    if outermost && key == b"info" {
                        self.info_span = Some(value_start..self.pos);
                    }
                    if map.insert(key, value).is_some() {
                        return Err(Error::DuplicateKey(key_at));
                    }
                }
                Some(_) => return Err(Error::NonStringKey(self.pos)),
            }
        }
    }

    fn find(&self, needle: u8) -> Option<usize> {
        self.input[self.pos..]
            .iter()
            .position(|b| *b == needle)
            .map(|i| i + self.pos)
    }
}

/// Encode a value canonically.
pub fn encode(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    encode_into(value, &mut out);
    out
}

/// Encode a value canonically, appending to `out`.
pub fn encode_into(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Bytes(bytes) => {
            out.extend_from_slice(bytes.len().to_string().as_bytes());
            out.push(b':');
            out.extend_from_slice(bytes);
        }
        Value::Int(n) => {
            out.push(b'i');
            out.extend_from_slice(n.to_string().as_bytes());
            out.push(b'e');
        }
        Value::List(items) => {
            out.push(b'l');
            for item in items {
                encode_into(item, out);
            }
            out.push(b'e');
        }
        Value::Dict(map) => {
            out.push(b'd');
            // BTreeMap iterates in sorted key order, which is exactly what
            // BEP 3 requires.
            for (key, item) in map {
                encode_into(&Value::Bytes(key.clone()), out);
                encode_into(item, out);
            }
            out.push(b'e');
        }
    }
}

impl Value {
    /// A byte string from anything string-like.
    pub fn text(value: impl Into<String>) -> Self {
        Self::Bytes(value.into().into_bytes())
    }

    /// The dictionary entry at `key`, when this is a dictionary.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Self::Dict(map) => map.get(key.as_bytes()),
            _ => None,
        }
    }

    /// This value as a dictionary.
    pub fn as_dict(&self) -> Option<&BTreeMap<Vec<u8>, Value>> {
        match self {
            Self::Dict(map) => Some(map),
            _ => None,
        }
    }

    /// This value as a mutable dictionary.
    pub fn as_dict_mut(&mut self) -> Option<&mut BTreeMap<Vec<u8>, Value>> {
        match self {
            Self::Dict(map) => Some(map),
            _ => None,
        }
    }

    /// This value as a list.
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }

    /// This value as raw bytes.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// This value as an integer.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// This value as UTF-8 text, lossily.
    ///
    /// Torrent metadata is byte strings, and plenty of real torrents carry
    /// names that are not valid UTF-8. Refusing to display them would be worse
    /// than showing a replacement character, so this never fails.
    pub fn as_text(&self) -> Option<String> {
        self.as_bytes()
            .map(|b| String::from_utf8_lossy(b).into_owned())
    }

    /// A list of byte strings as text, skipping anything that is not a string.
    pub fn as_text_list(&self) -> Vec<String> {
        self.as_list()
            .map(|items| items.iter().filter_map(Value::as_text).collect())
            .unwrap_or_default()
    }

    /// One byte string, or a list of them, as text.
    ///
    /// Several metainfo keys are specified as a list and are written in the
    /// wild as a bare string when there is only one entry. A reader that
    /// accepts the list alone returns nothing for those, with no error, so
    /// every key with that history reads through here rather than through
    /// [`Value::as_text_list`] directly.
    pub fn as_text_or_text_list(&self) -> Vec<String> {
        match self {
            Self::Bytes(_) => self.as_text().into_iter().collect(),
            _ => self.as_text_list(),
        }
    }
}

impl fmt::Display for Value {
    /// A compact, readable rendering for diagnostics. Not bencode.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bytes(bytes) => match std::str::from_utf8(bytes) {
                Ok(text) if text.chars().all(|c| !c.is_control()) => write!(f, "{text:?}"),
                _ => write!(f, "<{} bytes>", bytes.len()),
            },
            Self::Int(n) => write!(f, "{n}"),
            Self::List(items) => {
                let rendered: Vec<String> = items.iter().map(ToString::to_string).collect();
                write!(f, "[{}]", rendered.join(", "))
            }
            Self::Dict(map) => {
                let rendered: Vec<String> = map
                    .iter()
                    .map(|(k, v)| format!("{}: {v}", String::from_utf8_lossy(k)))
                    .collect();
                write!(f, "{{{}}}", rendered.join(", "))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dict(pairs: &[(&str, Value)]) -> Value {
        Value::Dict(
            pairs
                .iter()
                .map(|(k, v)| (k.as_bytes().to_vec(), v.clone()))
                .collect(),
        )
    }

    #[test]
    fn integers_round_trip() {
        for (encoded, value) in [("i0e", 0i64), ("i42e", 42), ("i-7e", -7)] {
            assert_eq!(decode(encoded.as_bytes()).unwrap(), Value::Int(value));
            assert_eq!(encode(&Value::Int(value)), encoded.as_bytes());
        }
    }

    #[test]
    fn non_canonical_integers_are_refused() {
        for bad in ["i03e", "i-0e", "i0042e"] {
            assert!(
                matches!(decode(bad.as_bytes()), Err(Error::NonCanonicalInteger(_))),
                "{bad} should be refused"
            );
        }
    }

    #[test]
    fn byte_strings_round_trip_including_binary() {
        assert_eq!(decode(b"4:spam").unwrap(), Value::Bytes(b"spam".to_vec()));
        assert_eq!(decode(b"0:").unwrap(), Value::Bytes(Vec::new()));
        let binary = Value::Bytes(vec![0, 255, 128, b':']);
        assert_eq!(decode(&encode(&binary)).unwrap(), binary);
    }

    #[test]
    fn a_length_longer_than_the_input_is_refused() {
        assert!(matches!(
            decode(b"10:short"),
            Err(Error::LengthOverrun { .. })
        ));
        assert!(matches!(decode(b"04:spam"), Err(Error::BadLength(_))));
    }

    #[test]
    fn lists_round_trip_and_nest() {
        let value = Value::List(vec![Value::Int(1), Value::text("a"), Value::List(vec![])]);
        assert_eq!(encode(&value), b"li1e1:alee");
        assert_eq!(decode(&encode(&value)).unwrap(), value);
    }

    #[test]
    fn dictionary_keys_are_emitted_in_sorted_order() {
        // Inserted out of order, emitted in byte order.
        let value = dict(&[("zebra", Value::Int(1)), ("apple", Value::Int(2))]);
        assert_eq!(encode(&value), b"d5:applei2e5:zebrai1ee");
    }

    #[test]
    fn key_sorting_is_by_raw_bytes_not_by_length() {
        // "b" sorts after "ab" by byte value even though it is shorter.
        let value = dict(&[("b", Value::Int(1)), ("ab", Value::Int(2))]);
        assert_eq!(encode(&value), b"d2:abi2e1:bi1ee");
    }

    #[test]
    fn encoding_is_stable_regardless_of_insertion_order() {
        let one = dict(&[
            ("a", Value::Int(1)),
            ("b", Value::Int(2)),
            ("c", Value::Int(3)),
        ]);
        let other = dict(&[
            ("c", Value::Int(3)),
            ("a", Value::Int(1)),
            ("b", Value::Int(2)),
        ]);
        assert_eq!(encode(&one), encode(&other));
    }

    #[test]
    fn a_duplicate_key_is_refused() {
        assert!(matches!(
            decode(b"d1:ai1e1:ai2ee"),
            Err(Error::DuplicateKey(_))
        ));
    }

    #[test]
    fn a_non_string_key_is_refused() {
        assert!(matches!(decode(b"di1ei2ee"), Err(Error::NonStringKey(_))));
    }

    #[test]
    fn trailing_data_is_refused() {
        assert!(matches!(
            decode(b"i1eXX"),
            Err(Error::TrailingData { trailing: 2, .. })
        ));
        // A prefix decode accepts it and reports where it stopped.
        assert_eq!(decode_prefix(b"i1eXX").unwrap(), (Value::Int(1), 3));
    }

    #[test]
    fn truncated_input_is_refused_rather_than_silently_accepted() {
        let truncated: [&[u8]; 5] = [b"d1:a", b"li1e", b"i42", b"3:ab", b"d"];
        for input in truncated {
            assert!(decode(input).is_err(), "{input:?} should be refused");
        }
    }

    #[test]
    fn the_info_span_is_the_exact_bytes_of_the_info_value() {
        let torrent = b"d8:announce3:foo4:infod4:name3:bar12:piece lengthi16eee";
        let (_, span) = decode_with_info_span(torrent).unwrap();
        let span = span.expect("info key exists");
        assert_eq!(&torrent[span.clone()], b"d4:name3:bar12:piece lengthi16ee");
        // The span decodes on its own, which is what SHA-1 is taken over.
        assert!(decode(&torrent[span]).is_ok());
    }

    #[test]
    fn a_nested_info_key_is_not_mistaken_for_the_top_level_one() {
        // The inner dict under key "a" also has an "info" key. Only the outer
        // one counts, because that is the one the info hash is taken over.
        let torrent = b"d1:ad4:info3:xxxe4:infod4:name3:baree";
        let (_, span) = decode_with_info_span(torrent).unwrap();
        let span = span.expect("top level info exists");
        assert_eq!(&torrent[span], b"d4:name3:bare");
    }

    #[test]
    fn a_torrent_without_an_info_key_reports_no_span() {
        let (_, span) = decode_with_info_span(b"d8:announce3:fooe").unwrap();
        assert!(span.is_none());
    }

    #[test]
    fn accessors_read_what_is_there_and_nothing_else() {
        let value = dict(&[
            ("n", Value::Int(7)),
            ("s", Value::text("hi")),
            ("l", Value::List(vec![Value::text("a"), Value::Int(1)])),
        ]);
        assert_eq!(value.get("n").and_then(Value::as_int), Some(7));
        assert_eq!(
            value.get("s").and_then(Value::as_text).as_deref(),
            Some("hi")
        );
        assert_eq!(
            value.get("l").map(Value::as_text_list),
            Some(vec!["a".to_string()])
        );
        assert!(value.get("missing").is_none());
        assert!(value.get("n").and_then(Value::as_bytes).is_none());
    }

    #[test]
    fn one_string_and_a_list_of_them_read_the_same_way() {
        let value = dict(&[
            ("one", Value::text("https://a.example.com/")),
            (
                "many",
                Value::List(vec![
                    Value::text("https://a.example.com/"),
                    Value::text("https://b.example.com/"),
                    Value::Int(1),
                ]),
            ),
            ("neither", Value::Int(7)),
        ]);
        assert_eq!(
            value.get("one").map(Value::as_text_or_text_list),
            Some(vec!["https://a.example.com/".to_string()])
        );
        assert_eq!(
            value.get("many").map(Value::as_text_or_text_list),
            Some(vec![
                "https://a.example.com/".to_string(),
                "https://b.example.com/".to_string(),
            ])
        );
        // A shape that is neither still yields nothing rather than panicking,
        // and the plain list accessor keeps refusing the string form, which is
        // why the two are separate methods.
        assert_eq!(
            value.get("neither").map(Value::as_text_or_text_list),
            Some(Vec::new())
        );
        assert_eq!(value.get("one").map(Value::as_text_list), Some(Vec::new()));
    }

    #[test]
    fn invalid_utf8_names_are_shown_rather_than_refused() {
        let value = Value::Bytes(vec![0xff, 0xfe]);
        assert!(value.as_text().is_some());
    }

    #[test]
    fn a_realistic_torrent_round_trips_byte_for_byte() {
        let original = dict(&[
            ("announce", Value::text("udp://tracker.example.com:80")),
            (
                "announce-list",
                Value::List(vec![Value::List(vec![Value::text("udp://a:80")])]),
            ),
            ("comment", Value::text("hello")),
            ("creation date", Value::Int(1_787_140_323)),
            (
                "info",
                dict(&[
                    ("length", Value::Int(1024)),
                    ("name", Value::text("payload.bin")),
                    ("piece length", Value::Int(16384)),
                    ("pieces", Value::Bytes(vec![0u8; 20])),
                ]),
            ),
            (
                "url-list",
                Value::List(vec![Value::text("https://e.com/pub/")]),
            ),
        ]);
        let encoded = encode(&original);
        assert_eq!(decode(&encoded).unwrap(), original);
        assert_eq!(encode(&decode(&encoded).unwrap()), encoded);
    }
}
