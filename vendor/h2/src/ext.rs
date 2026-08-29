//! Extensions specific to the HTTP/2 protocol.

use crate::hpack::BytesStr;

use bytes::Bytes;
use std::fmt;

/// Represents the `:protocol` pseudo-header used by
/// the [Extended CONNECT Protocol].
///
/// [Extended CONNECT Protocol]: https://datatracker.ietf.org/doc/html/rfc8441#section-4
#[derive(Clone, Eq, PartialEq)]
pub struct Protocol {
    value: BytesStr,
}

impl Protocol {
    /// Converts a static string to a protocol name.
    pub const fn from_static(value: &'static str) -> Self {
        Self {
            value: BytesStr::from_static(value),
        }
    }

    /// Returns a str representation of the header.
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    pub(crate) fn try_from(bytes: Bytes) -> Result<Self, std::str::Utf8Error> {
        Ok(Self {
            value: BytesStr::try_from(bytes)?,
        })
    }
}

impl<'a> From<&'a str> for Protocol {
    fn from(value: &'a str) -> Self {
        Self {
            value: BytesStr::from(value),
        }
    }
}

impl AsRef<[u8]> for Protocol {
    fn as_ref(&self) -> &[u8] {
        self.value.as_ref()
    }
}

impl fmt::Debug for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

/// One of the six HTTP/2 pseudo-header fields.
///
/// See [`PseudoOrder`], which is what this exists for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PseudoName {
    /// `:method`
    Method,
    /// `:scheme`
    Scheme,
    /// `:authority`
    Authority,
    /// `:path`
    Path,
    /// `:protocol`, from the [Extended CONNECT Protocol].
    ///
    /// [Extended CONNECT Protocol]: https://datatracker.ietf.org/doc/html/rfc8441#section-4
    Protocol,
    /// `:status`
    Status,
}

impl PseudoName {
    /// Every pseudo-header, in the order this crate has always written them.
    pub const ALL: [PseudoName; 6] = [
        PseudoName::Method,
        PseudoName::Scheme,
        PseudoName::Authority,
        PseudoName::Path,
        PseudoName::Protocol,
        PseudoName::Status,
    ];

    /// The wire name, colon included.
    pub const fn as_str(self) -> &'static str {
        match self {
            PseudoName::Method => ":method",
            PseudoName::Scheme => ":scheme",
            PseudoName::Authority => ":authority",
            PseudoName::Path => ":path",
            PseudoName::Protocol => ":protocol",
            PseudoName::Status => ":status",
        }
    }

    /// Parse a wire name, colon included. Case sensitive, because a
    /// pseudo-header name is lowercase on the wire.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            ":method" => Some(PseudoName::Method),
            ":scheme" => Some(PseudoName::Scheme),
            ":authority" => Some(PseudoName::Authority),
            ":path" => Some(PseudoName::Path),
            ":protocol" => Some(PseudoName::Protocol),
            ":status" => Some(PseudoName::Status),
            _ => None,
        }
    }
}

/// The order the pseudo-header fields are written into a HEADERS frame.
///
/// RFC 9113 section 8.3 requires the pseudo-headers to precede the regular
/// fields and says nothing about the order among themselves, so a client is
/// free to choose one. That freedom is observable: the sequence is part of
/// what an origin reads to tell one client from another, and every
/// implementation has its own. This crate's own order is [`Default`], and
/// nothing changes unless a caller asks for something else.
///
/// Set it per request by putting one in the request's extensions, the same way
/// [`Protocol`] is set:
///
/// ```
/// # use h2::ext::{PseudoName, PseudoOrder};
/// let order = PseudoOrder::new([
///     PseudoName::Method,
///     PseudoName::Authority,
///     PseudoName::Scheme,
///     PseudoName::Path,
///     PseudoName::Protocol,
///     PseudoName::Status,
/// ]);
/// let request = http::Request::builder()
///     .uri("https://example.com/")
///     .extension(order)
///     .body(())
///     .unwrap();
/// ```
///
/// A name that names no pseudo-header this request carries is skipped, so an
/// order is safe to apply to any request. Every name appears exactly once, by
/// construction, so a value that would drop or duplicate a pseudo-header
/// cannot be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PseudoOrder {
    inner: [PseudoName; 6],
}

impl PseudoOrder {
    /// An order over all six names.
    ///
    /// The array is a permutation by type: six elements, and
    /// [`PseudoOrder::parse`] is the constructor that has to check.
    pub const fn new(order: [PseudoName; 6]) -> Self {
        Self { inner: order }
    }

    /// The names, in the order they are written.
    pub fn as_slice(&self) -> &[PseudoName] {
        &self.inner
    }

    /// Parse a comma separated list of wire names, colons included, such as
    /// `":method,:authority,:scheme,:path,:protocol,:status"`.
    ///
    /// Every name has to appear exactly once. A list that omits one would
    /// silently drop a pseudo-header from the frame, and a list that repeats
    /// one is a caller who does not know what they asked for.
    pub fn parse(text: &str) -> Result<Self, InvalidPseudoOrder> {
        let mut seen = [false; 6];
        let mut order = [PseudoName::Method; 6];
        let mut count = 0usize;
        for part in text.split(',') {
            let part = part.trim();
            let name = PseudoName::parse(part).ok_or_else(|| InvalidPseudoOrder {
                reason: format!("{part:?} is not a pseudo-header name"),
            })?;
            let slot = PseudoName::ALL
                .iter()
                .position(|candidate| *candidate == name)
                .expect("every PseudoName is in ALL");
            if seen[slot] {
                return Err(InvalidPseudoOrder {
                    reason: format!("{} appears more than once", name.as_str()),
                });
            }
            seen[slot] = true;
            if count == 6 {
                return Err(InvalidPseudoOrder {
                    reason: "more than six names".to_string(),
                });
            }
            order[count] = name;
            count += 1;
        }
        if count != 6 {
            let missing: Vec<&str> = PseudoName::ALL
                .iter()
                .enumerate()
                .filter(|(index, _)| !seen[*index])
                .map(|(_, name)| name.as_str())
                .collect();
            return Err(InvalidPseudoOrder {
                reason: format!("missing {}", missing.join(", ")),
            });
        }
        Ok(Self { inner: order })
    }
}

impl Default for PseudoOrder {
    fn default() -> Self {
        Self {
            inner: PseudoName::ALL,
        }
    }
}

/// What [`PseudoOrder::parse`] refused, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidPseudoOrder {
    reason: String,
}

impl fmt::Display for InvalidPseudoOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid pseudo-header order: {}", self.reason)
    }
}

impl std::error::Error for InvalidPseudoOrder {}
