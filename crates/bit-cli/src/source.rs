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
            return Ok(Self::Url(trimmed.to_string()));
        }
        // A bare info hash is 40 hex or 32 base32 characters and nothing else.
        // Checked before the path branch so a hash is never taken for a
        // relative filename.
        if (trimmed.len() == 40 && trimmed.chars().all(|c| c.is_ascii_hexdigit()))
            || (trimmed.len() == 32 && trimmed.chars().all(|c| c.is_ascii_alphanumeric()))
        {
            if let Ok(hash) = InfoHash::parse(trimmed) {
                return Ok(Self::InfoHash(hash));
            }
        }
        let path = env.resolve(Path::new(trimmed));
        if lower.ends_with(".meta4") || lower.ends_with(".metalink") {
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
    pub const fn needs_network(&self) -> bool {
        matches!(self, Self::Url(_) | Self::Magnet(_) | Self::InfoHash(_))
    }
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

/// Fetch a `.torrent` over HTTP.
pub async fn fetch_torrent(url: &str, user_agent: &str) -> Result<Metainfo> {
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
    let bytes = response
        .bytes()
        .await
        .map_err(|e| Error::network(format!("cannot read the body of {url}: {e}")))?;
    Metainfo::parse(&bytes).map_err(|e| Error::source_resolution(format!("{url}: {e}")))
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
            Kind::Stdin.name(),
        ];
        for name in names {
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{name}"
            );
        }
    }
}
