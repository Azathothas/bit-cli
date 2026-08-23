//! Resolving a `SOURCE` argument into torrent metadata.
//!
//! A source is one of:
//!
//! - a path to a `.torrent`
//! - an HTTP(S) URL to a `.torrent`
//! - a magnet URI
//! - a bare 40-character hex or 32-character base32 info hash
//! - a Metalink file (`.meta4` or `.metalink`)
//! - `-` for stdin
//!
//! A magnet or a bare info hash carries no piece hashes, so it has to be
//! resolved against the swarm before the torrent's shape is known. Commands
//! that need only the info hash work from it directly; commands that need the
//! file layout say so rather than resolving silently, because a swarm lookup
//! is a network operation with a very different cost from reading a file.

use std::io::Read;
use std::path::{Path, PathBuf};

use bit_cli_core::error::{Error, Result, from_io};
use bit_cli_core::metalink::{Metalink, MetalinkFile, Mirror};
use bit_cli_core::torrent::{InfoHash, Magnet, Metainfo};

use crate::env::Env;

/// What a `SOURCE` argument turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// A local `.torrent`.
    File(PathBuf),
    /// An HTTP(S) URL pointing at a `.torrent`.
    Url(String),
    /// A magnet URI.
    Magnet(Box<Magnet>),
    /// A bare info hash.
    InfoHash(InfoHash),
    /// A local Metalink.
    Metalink(PathBuf),
    /// An HTTP(S) URL pointing at a Metalink.
    ///
    /// Every real Metalink is served over HTTP, and `MirrorBrain` generates one
    /// on demand for any file it publishes, so a URL is how a caller normally
    /// meets one. A local `.meta4` is what you get after saving it by hand.
    /// See `TODO/cli-surface.md`, T-154.
    MetalinkUrl(String),
    /// Standard input.
    Stdin,
}

impl Kind {
    /// Classify a source string without touching the network or the disk.
    ///
    /// Classification is by shape, not by probing, so it is fast and its
    /// result is what error messages talk about.
    pub fn classify(source: &str, env: &Env) -> Result<Self> {
        let trimmed = source.trim();
        if trimmed == "-" {
            return Ok(Self::Stdin);
        }
        if trimmed.is_empty() {
            return Err(Error::source_resolution("empty source"));
        }
        if Magnet::looks_like(trimmed) {
            return Ok(Self::Magnet(Box::new(Magnet::parse(trimmed)?)));
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("http://") || lower.starts_with("https://") {
            // The extension is read from the **path**, not from the whole
            // string. `?file=x.meta4` is a query naming a file and
            // `#release.meta4` is a fragment, and neither says what the URL
            // itself serves. See `TODO/cli-surface.md`, T-154.
            return Ok(match is_metalink_name(url_path(&lower)) {
                true => Self::MetalinkUrl(trimmed.to_string()),
                false => Self::Url(trimmed.to_string()),
            });
        }
        // A bare info hash is 40 hex or 32 base32 characters and nothing else.
        // Checked before the path branch so a hash is never taken for a
        // relative filename.
        if ((trimmed.len() == 40 && trimmed.chars().all(|c| c.is_ascii_hexdigit()))
            || (trimmed.len() == 32 && trimmed.chars().all(|c| c.is_ascii_alphanumeric())))
            && let Ok(hash) = InfoHash::parse(trimmed)
        {
            return Ok(Self::InfoHash(hash));
        }
        let path = env.resolve(Path::new(trimmed));
        if is_metalink_name(&lower) {
            return Ok(Self::Metalink(path));
        }
        Ok(Self::File(path))
    }

    /// A short name for the kind, for output and error context.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::File(_) => "torrent_file",
            Self::Url(_) => "torrent_url",
            Self::Magnet(_) => "magnet",
            Self::InfoHash(_) => "info_hash",
            Self::Metalink(_) => "metalink",
            Self::MetalinkUrl(_) => "metalink_url",
            Self::Stdin => "stdin",
        }
    }

    /// The info hash, when the source names one without any lookup.
    pub fn info_hash(&self) -> Option<InfoHash> {
        match self {
            Self::InfoHash(hash) => Some(*hash),
            Self::Magnet(magnet) => magnet.info_hash,
            _ => None,
        }
    }

    /// Whether resolving this to full metadata needs the network.
    ///
    /// A Metalink is a local file and still needs it: the document names its
    /// `.torrent` by URL, and the layout is not known until that URL has been
    /// fetched. What is readable without the network is the document's own
    /// claims, which is what `--dry-run` reports.
    pub const fn needs_network(&self) -> bool {
        matches!(
            self,
            Self::Url(_)
                | Self::Magnet(_)
                | Self::InfoHash(_)
                | Self::Metalink(_)
                | Self::MetalinkUrl(_)
        )
    }

    /// Whether even the document's own claims need the network.
    ///
    /// A local Metalink is readable with nothing running, which is what
    /// `--dry-run` reports. One named by URL is not: the document itself is the
    /// thing that has to be fetched. See `TODO/cli-surface.md`, T-154.
    pub const fn document_needs_network(&self) -> bool {
        matches!(self, Self::MetalinkUrl(_))
    }
}

/// The path part of an already-lower-cased URL.
///
/// Everything from after the authority to the first `?` or `#`. A URL with no
/// path gives the empty string, which no extension matches.
fn url_path(lower: &str) -> &str {
    let after_scheme = lower.split_once("://").map_or(lower, |(_, rest)| rest);
    let path = match after_scheme.find('/') {
        Some(slash) => &after_scheme[slash..],
        None => "",
    };
    let end = path.find(['?', '#']).unwrap_or(path.len());
    &path[..end]
}

/// Whether an already-lower-cased name is a Metalink by extension.
fn is_metalink_name(lower: &str) -> bool {
    lower.ends_with(".meta4") || lower.ends_with(".metalink")
}

/// Load full metadata for a source that carries it.
///
/// A magnet or a bare info hash has no piece hashes, so this returns an error
/// naming what would be needed instead of quietly starting a swarm lookup.
/// Commands that can do the lookup call the engine directly.
pub fn load_local(kind: &Kind, env: &mut Env) -> Result<Metainfo> {
    match kind {
        Kind::File(path) => Metainfo::read(path),
        Kind::Stdin => {
            let mut bytes = Vec::new();
            std::io::stdin()
                .read_to_end(&mut bytes)
                .map_err(|e| from_io(e, "cannot read the torrent from stdin"))?;
            Metainfo::parse(&bytes)
        }
        Kind::Metalink(path) => Err(Error::source_resolution(format!(
            "{}: a metalink has to be resolved to its torrent first",
            path.display()
        ))
        .with("source_kind", "metalink")),
        Kind::MetalinkUrl(url) => Err(Error::source_resolution(format!(
            "{url}: a metalink has to be fetched and resolved to its torrent first"
        ))
        .with("source_kind", "metalink_url")),
        Kind::Url(url) => {
            let _ = env;
            Err(Error::source_resolution(format!(
                "{url} has to be fetched before it can be read"
            ))
            .with("source_kind", "torrent_url"))
        }
        Kind::Magnet(_) | Kind::InfoHash(_) => Err(Error::source_resolution(
            "a magnet URI and a bare info hash carry no piece hashes, so the metadata has to be resolved from the swarm first",
        )
        .with("source_kind", kind.name())
        .with("hint", "use `bit-cli download` or supply the .torrent")),
    }
}

/// Fetch a `.torrent` over HTTP, keeping the bytes as well as the parse.
///
/// Both, because the caller hands the exact bytes to the session rather than
/// handing it the URL again. Fetching a URL twice can return two different
/// documents, and a run whose report describes one torrent while the session
/// downloads another is the worst kind of wrong answer.
pub async fn fetch_torrent(url: &str, user_agent: &str) -> Result<(Metainfo, Vec<u8>)> {
    let bytes = fetch_bytes(url, user_agent).await?;
    let meta =
        Metainfo::parse(&bytes).map_err(|e| Error::source_resolution(format!("{url}: {e}")))?;
    Ok((meta, bytes))
}

/// Fetch a URL and return its body, failing on any status that is not success.
async fn fetch_bytes(url: &str, user_agent: &str) -> Result<Vec<u8>> {
    let client = reqwest_client(user_agent)?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| Error::network(format!("cannot fetch {url}: {e}")))?;
    let status = response.status();
    if !status.is_success() {
        return Err(Error::source_resolution(format!("{url}: {status}"))
            .with("url", url.to_string())
            .with("http_status", status.as_u16()));
    }
    response
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| Error::network(format!("cannot read the body of {url}: {e}")))
}

/// A Metalink read, and the `.torrent` it names fetched.
///
/// See `TODO/cli-surface.md`, T-113.
#[derive(Debug)]
pub struct ResolvedMetalink {
    /// `4` or `3`.
    pub version: &'static str,
    /// The one `<file>` the document is about.
    pub file: MetalinkFile,
    /// The `<metaurl>` the `.torrent` actually came from.
    pub torrent_url: String,
    /// Torrent URLs that were tried before it, and what went wrong with each.
    /// Empty when the document's first choice answered.
    pub torrent_errors: Vec<(String, String)>,
    /// The exact bytes fetched. Handed to the session unchanged.
    pub torrent_bytes: Vec<u8>,
    pub meta: Metainfo,
}

/// Largest Metalink document accepted over HTTP, in bytes.
///
/// A `MirrorBrain` document with a hundred mirrors is a few kilobytes. The
/// ceiling exists because the URL comes from the caller and the body comes from
/// whoever answers it, which is the same reason `MAX_LIST_BYTES` exists, and it
/// is the same number for the same reason.
const MAX_METALINK_BYTES: usize = 1024 * 1024;

/// Fetch a Metalink over HTTP and parse it.
///
/// The remote half of [`Kind::MetalinkUrl`]. Everything after this is what a
/// local `.meta4` already does: [`resolve_metalink`] takes the parsed document
/// and neither knows nor cares where it came from.
///
/// **Nothing is resolved relative to the document's URL.** A `MirrorBrain`
/// document is generated per request and its `<origin dynamic="true">` names
/// the URL it came from, so every `<url>` and `<metaurl>` in it is absolute.
/// A document with relative mirror URLs would need a base to resolve against,
/// and `bit-cli` refuses those on both paths rather than resolving one kind
/// and not the other. See `TODO/cli-surface.md`, T-154.
pub async fn fetch_metalink(url: &str, user_agent: &str) -> Result<Metalink> {
    let bytes = fetch_bytes(url, user_agent).await?;
    if bytes.len() > MAX_METALINK_BYTES {
        return Err(Error::source_resolution(format!(
            "{url}: the metalink is {} bytes, and the ceiling is {MAX_METALINK_BYTES}",
            bytes.len()
        ))
        .with("url", url.to_string())
        .with("bytes", bytes.len()));
    }
    Metalink::parse(&bytes).map_err(|e| Error::source_resolution(format!("{url}: {e}")))
}

/// Read a Metalink and fetch the torrent it names.
///
/// The document may list several torrents, which is a mirror list for the
/// `.torrent` itself. They are tried in the document's own preferred order and
/// the first that parses wins; the failures are kept so the report can say the
/// preferred one was not the one used.
pub async fn resolve_metalink(document: &Metalink, user_agent: &str) -> Result<ResolvedMetalink> {
    let file = document.single_file()?.clone();
    // Owned, because the fetch below moves `file` into the result while the
    // loop is still walking the list.
    let torrents: Vec<Mirror> = file.torrents_by_priority().into_iter().cloned().collect();
    if torrents.is_empty() {
        return Err(Error::source_resolution(format!(
            "the metalink lists no torrent for {}, so there is nothing to download here. It lists {} HTTP mirror(s); pass one with --web-seed against a .torrent you already have.",
            match file.name.is_empty() {
                true => "its file".to_string(),
                false => file.name.clone(),
            },
            file.mirrors.len()
        ))
        .with("source_kind", "metalink")
        .with("mirrors", file.mirrors.len()));
    }
    let mut torrent_errors = Vec::new();
    for mirror in &torrents {
        match fetch_torrent(&mirror.url, user_agent).await {
            Ok((meta, torrent_bytes)) => {
                return Ok(ResolvedMetalink {
                    version: document.version.as_str(),
                    file,
                    torrent_url: mirror.url.clone(),
                    torrent_errors,
                    torrent_bytes,
                    meta,
                });
            }
            Err(error) => torrent_errors.push((mirror.url.clone(), error.to_string())),
        }
    }
    let tried = torrent_errors.len();
    let detail: Vec<String> = torrent_errors
        .iter()
        .map(|(url, error)| format!("{url}: {error}"))
        .collect();
    Err(Error::source_resolution(format!(
        "none of the {tried} torrent(s) the metalink lists could be fetched: {}",
        detail.join("; ")
    ))
    .with("source_kind", "metalink")
    .with("torrents_tried", tried))
}

/// Longest a list fetch may take, connect and body together.
///
/// A tracker list that never finishes arriving must not hold up a download
/// that has everything else it needs.
const LIST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Largest list body accepted, in bytes.
///
/// A tracker list is a few kilobytes. One megabyte is four orders of magnitude
/// of headroom and still bounds a URL that answers with a payload.
const MAX_LIST_BYTES: usize = 1024 * 1024;

/// Fetch a plain-text list over HTTP.
///
/// Backs `--tracker-list-url` and `--web-seed-list-url`. The URL comes from the
/// caller and the response comes from whoever answers it, so neither is
/// trusted: the scheme has to be HTTP or HTTPS, the whole exchange is under a
/// deadline, and the body is capped.
///
/// A body that exceeds the cap is refused rather than truncated. Half a
/// tracker list is a run announcing to a set of trackers nobody chose, and a
/// truncated last line is a URL that is not the URL anyone wrote.
///
/// See `TODO/cli-surface.md`, T-181 and T-183.
pub async fn fetch_list(url: &str, user_agent: &str) -> Result<String> {
    // The scheme is matched on the raw string rather than through a URL
    // parser, because the only question here is whether this is an HTTP URL
    // and a parser would be a dependency to answer it. Schemes are
    // case-insensitive, so `HTTPS://` is the same URL.
    let scheme_is_http = {
        let lower = url.to_ascii_lowercase();
        lower.starts_with("http://") || lower.starts_with("https://")
    };
    if !scheme_is_http {
        return Err(Error::usage(format!(
            "{url} is not an HTTP URL, and a list is fetched over HTTP only"
        ))
        .with("url", url.to_string()));
    }

    let client = reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(LIST_TIMEOUT)
        .build()
        .map_err(|e| Error::network(format!("cannot build an HTTP client: {e}")))?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| Error::network(format!("cannot fetch {url}: {e}")))?;
    let status = response.status();
    if !status.is_success() {
        return Err(Error::network(format!("{url}: {status}"))
            .with("url", url.to_string())
            .with("http_status", status.as_u16()));
    }

    // Read in chunks rather than calling `bytes()`, so the cap bounds what is
    // held in memory rather than only what is returned. A server declaring a
    // small `Content-Length` and sending more would otherwise be read in full
    // before anything checked it.
    let mut response = response;
    let mut body: Vec<u8> = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| Error::network(format!("cannot read the body of {url}: {e}")))?
    {
        body.extend_from_slice(&chunk);
        if body.len() > MAX_LIST_BYTES {
            return Err(Error::network(format!(
                "{url} answered with more than {MAX_LIST_BYTES} bytes, which is not a list"
            ))
            .with("url", url.to_string())
            .with("max_bytes", MAX_LIST_BYTES));
        }
    }

    String::from_utf8(body).map_err(|_| {
        Error::network(format!("{url} answered with bytes that are not UTF-8 text"))
            .with("url", url.to_string())
    })
}

/// A list fetcher that runs on an existing runtime.
///
/// The list flags are read while the command is still synchronous, so the
/// fetch borrows the runtime the command has already built rather than
/// building one of its own. Building a second runtime inside the first is what
/// this avoids.
pub fn list_fetcher<'a>(
    runtime: &'a tokio::runtime::Runtime,
    user_agent: &'a str,
) -> impl Fn(&str) -> Result<String> + 'a {
    move |url: &str| runtime.block_on(fetch_list(url, user_agent))
}

fn reqwest_client(user_agent: &str) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(user_agent)
        .build()
        .map_err(|e| Error::network(format!("cannot build an HTTP client: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEX: &str = "0102030405060708090a0b0c0d0e0f1011121314";

    fn env() -> Env {
        Env::test(&[], "/work").0
    }

    #[test]
    fn a_dash_is_stdin() {
        assert_eq!(Kind::classify("-", &env()).unwrap(), Kind::Stdin);
    }

    #[test]
    fn an_http_url_is_a_url() {
        let kind = Kind::classify("https://e.com/x.torrent", &env()).unwrap();
        assert_eq!(kind, Kind::Url("https://e.com/x.torrent".to_string()));
        assert!(kind.needs_network());
    }

    #[test]
    fn a_magnet_is_parsed_at_classification_time() {
        let kind = Kind::classify(&format!("magnet:?xt=urn:btih:{HEX}"), &env()).unwrap();
        assert_eq!(kind.name(), "magnet");
        assert_eq!(kind.info_hash().unwrap().hex(), HEX);
    }

    #[test]
    fn a_bare_hex_hash_is_an_info_hash_not_a_filename() {
        let kind = Kind::classify(HEX, &env()).unwrap();
        assert_eq!(kind, Kind::InfoHash(InfoHash::parse(HEX).unwrap()));
    }

    #[test]
    fn a_bare_base32_hash_is_an_info_hash() {
        let kind = Kind::classify("AEBAGBAFAYDQQCIKBMGA2DQPCAIREEYU", &env()).unwrap();
        assert_eq!(kind.info_hash().unwrap().hex(), HEX);
    }

    #[test]
    fn a_forty_character_filename_that_is_not_hex_stays_a_path() {
        let name = "z".repeat(40);
        assert!(matches!(
            Kind::classify(&name, &env()).unwrap(),
            Kind::File(_)
        ));
    }

    #[test]
    fn a_relative_path_resolves_against_the_working_directory() {
        let kind = Kind::classify("sub/x.torrent", &env()).unwrap();
        assert_eq!(kind, Kind::File(PathBuf::from("/work/sub/x.torrent")));
    }

    #[test]
    fn metalink_extensions_are_recognised() {
        for name in ["r.meta4", "r.metalink", "R.META4"] {
            assert!(
                matches!(Kind::classify(name, &env()).unwrap(), Kind::Metalink(_)),
                "{name} should be a metalink"
            );
        }
    }

    /// Every real Metalink is served over HTTP, so a URL is how a caller
    /// normally meets one. Until T-154 the `http://` prefix was checked before
    /// the extension, so this was a `Kind::Url`, was handed to the session as a
    /// `.torrent`, and failed on the bencode parse with a message about the
    /// torrent rather than about the metalink.
    #[test]
    fn a_metalink_named_by_url_is_a_metalink() {
        for url in [
            "https://e.com/r.meta4",
            "http://e.com/r.metalink",
            "HTTPS://E.COM/R.META4",
            "https://e.com/pub/25.8/r.msi.meta4",
        ] {
            assert_eq!(
                Kind::classify(url, &env()).unwrap(),
                Kind::MetalinkUrl(url.to_string()),
                "{url} should be a metalink URL"
            );
        }
    }

    /// The extension is read from the URL's **path**. A query naming a file and
    /// a fragment are not statements about what the URL serves, and a
    /// `MirrorBrain` instance generating a document per request is exactly the
    /// place a query string turns up. T-154.
    #[test]
    fn only_the_url_path_decides_whether_it_is_a_metalink() {
        for url in [
            // The extension is in the query, and the path is a torrent.
            "https://e.com/x.torrent?file=r.meta4",
            // In the fragment.
            "https://e.com/x.torrent#r.metalink",
            // No path at all.
            "https://e.com",
        ] {
            assert!(
                matches!(Kind::classify(url, &env()).unwrap(), Kind::Url(_)),
                "{url} should be a plain URL"
            );
        }
        // And the other way: the path ends in `.meta4` and the query is noise.
        assert!(matches!(
            Kind::classify("https://e.com/r.meta4?mirrorlist", &env()).unwrap(),
            Kind::MetalinkUrl(_)
        ));
    }

    /// A local Metalink is readable with nothing running; one named by URL is
    /// not, and `--dry-run` reports the difference rather than fetching. T-154.
    #[test]
    fn only_a_metalink_url_needs_the_network_to_read_the_document() {
        assert!(
            !Kind::classify("r.meta4", &env())
                .unwrap()
                .document_needs_network()
        );
        assert!(
            Kind::classify("https://e.com/r.meta4", &env())
                .unwrap()
                .document_needs_network()
        );
        // A plain torrent URL needs the network for the torrent, and there is
        // no separate document to read, so this is false rather than true.
        assert!(
            !Kind::classify("https://e.com/x.torrent", &env())
                .unwrap()
                .document_needs_network()
        );
    }

    #[test]
    fn an_empty_source_is_refused() {
        assert!(Kind::classify("", &env()).is_err());
        assert!(Kind::classify("   ", &env()).is_err());
    }

    #[test]
    fn only_remote_kinds_need_the_network() {
        assert!(!Kind::classify("x.torrent", &env()).unwrap().needs_network());
        assert!(!Kind::classify("-", &env()).unwrap().needs_network());
        assert!(
            Kind::classify("https://e.com/x.torrent", &env())
                .unwrap()
                .needs_network()
        );
        assert!(Kind::classify(HEX, &env()).unwrap().needs_network());
    }

    /// A metalink is a local file whose torrent is not. The layout is not
    /// known until the `<metaurl>` has been fetched, so a caller that skips
    /// the network cannot resolve one.
    #[test]
    fn a_metalink_is_a_local_file_that_still_needs_the_network() {
        let kind = Kind::classify("r.meta4", &env()).unwrap();
        assert!(matches!(kind, Kind::Metalink(_)));
        assert!(kind.needs_network());
    }

    #[test]
    fn loading_a_magnet_locally_says_why_it_cannot_work() {
        let mut env = env();
        let kind = Kind::classify(&format!("magnet:?xt=urn:btih:{HEX}"), &env).unwrap();
        let err = load_local(&kind, &mut env).unwrap_err();
        assert_eq!(err.code(), bit_cli_core::ExitCode::SourceResolution);
        assert!(
            err.message().contains("no piece hashes"),
            "{}",
            err.message()
        );
        assert!(err.context().contains_key("hint"));
    }

    #[test]
    fn every_kind_has_a_stable_name() {
        let names = [
            Kind::File(PathBuf::new()).name(),
            Kind::Url(String::new()).name(),
            Kind::InfoHash(InfoHash([0; 20])).name(),
            Kind::Metalink(PathBuf::new()).name(),
            Kind::MetalinkUrl(String::new()).name(),
            Kind::Stdin.name(),
        ];
        for name in names {
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{name}"
            );
        }
        // Distinct, because `kind` is what a caller branches on and two kinds
        // sharing a name is the same defect as none having one. The list above
        // is missing `Magnet` only because it holds a parsed magnet.
        let mut sorted = names.to_vec();
        sorted.sort_unstable();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), before, "two kinds share a name: {names:?}");
    }
}
