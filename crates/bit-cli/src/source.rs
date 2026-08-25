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
//! that need only the info hash work from it directly, and the rest join the
//! swarm the source names and ask a peer for the metadata over BEP 9.
//!
//! **That last part is the one thing here that is not a read.** A swarm lookup
//! is a different operation from a `GET`, with a different cost, so it has its
//! own flags rather than happening silently: `--peer`, `--no-dht`, `--no-lsd`
//! and `--no-tracker` under "Resolving a magnet" in any command's help. It is
//! not opt-in, for the reason [T-245](../../TODO/cli-surface.md) closed on: a
//! source kind one command accepts and four refuse is a defect rather than a
//! safeguard. See `TODO/metainfo.md`, T-241.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

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
        // A scheme this tree does not speak is named as one, rather than
        // falling through to "treat it as a path" and coming back as a
        // filename that no filesystem would accept. See
        // `TODO/cli-surface.md`, T-246.
        if let Some(scheme) = foreign_scheme(&lower) {
            return Err(Error::usage(format!(
                "`{scheme}:` is not a scheme this reads. A source is an http:// or https:// URL, a magnet: URI, a .torrent or metalink path, a bare info hash, or `-` for stdin"
            ))
            .with("scheme", scheme.to_string()));
        }
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

/// The scheme of a `scheme://` URL this tree does not speak, if that is what
/// the source is.
///
/// `http` and `https` return `None` because they are spoken, and `magnet:` is
/// classified before this is asked. The scheme has to be **two or more**
/// characters: a Windows drive letter is one character followed by a colon,
/// and `C://Users/x` is a path rather than a URL under a scheme called `C`.
///
/// Only the `://` form is tested. `mailto:x` and `urn:btih:...` carry no
/// authority, and the second of those is a shape a caller may reasonably paste
/// in, so neither is turned into an error here. See `TODO/cli-surface.md`,
/// T-246.
fn foreign_scheme(lower: &str) -> Option<&str> {
    let scheme = lower.split_once("://")?.0;
    if scheme.len() < 2 || scheme == "http" || scheme == "https" {
        return None;
    }
    let mut characters = scheme.chars();
    let first_is_alpha = characters.next().is_some_and(|c| c.is_ascii_alphabetic());
    let rest_is_scheme = characters.all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c));
    (first_is_alpha && rest_is_scheme).then_some(scheme)
}

/// Read a `.torrent` from disk, naming a directory as one rather than letting
/// the operating system describe it.
///
/// Opening a directory as a file is `ERROR_ACCESS_DENIED` on Windows and
/// `EISDIR` on Unix, so the same input produced two different explanations and
/// neither was true: nothing is denied and the caller almost always meant
/// `create`. See `TODO/cli-surface.md`, T-246.
pub fn read_torrent_file(path: &Path) -> Result<Metainfo> {
    if path.is_dir() {
        return Err(Error::usage(format!(
            "{} is a directory, not a .torrent. `bit-cli create` is the command that takes a directory",
            path.display()
        ))
        .with("path", path.display().to_string())
        .with("source_kind", "directory"));
    }
    Metainfo::read(path)
}

/// Load full metadata for a source that is already on this machine.
///
/// **This is the local-only path and it is not what a command should call.**
/// [`resolve_blocking`] is: it reads a local source through this and fetches a
/// remote one, which is what every command's `SOURCE` help text has always
/// promised. This stays for a caller that must not touch the network, and for
/// the two kinds that no single `GET` can answer.
///
/// A magnet or a bare info hash has no piece hashes, so this returns an error
/// naming what would be needed instead of quietly starting a swarm lookup.
/// Commands that can do the lookup call the engine directly.
pub fn load_local(kind: &Kind, env: &mut Env) -> Result<Metainfo> {
    match kind {
        Kind::File(path) => read_torrent_file(path),
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
        .with(
            "hint",
            "this is the local-only path; `resolve` joins the swarm and reads it",
        )),
    }
}

/// Resolve a source to full metadata, fetching it when it is remote.
///
/// **This is what a command calls**, and [`load_local`] is the half of it that
/// never touches the network. The difference between the two used to be the
/// difference between `download` and every other command: `download` handed a
/// URL to the session, which fetched it, and the nine commands that read the
/// torrent themselves refused the same URL their own help text offers. See
/// `TODO/cli-surface.md`, T-245.
///
/// A magnet and a bare info hash are resolved against the swarm they name,
/// which is [`resolve_from_swarm`]. That was a refusal until 2026-08-25 and
/// four commands exited 4 on a source kind a fifth accepted. See
/// `TODO/metainfo.md`, T-241.
pub async fn resolve(
    kind: &Kind,
    env: &mut Env,
    user_agent: &str,
    deadline: Duration,
    swarm: &SwarmResolve,
) -> Result<Metainfo> {
    match kind {
        Kind::Magnet(magnet) => resolve_from_swarm(&magnet.to_uri(), swarm, deadline).await,
        Kind::InfoHash(hash) => resolve_from_swarm(&hash.hex(), swarm, deadline).await,
        Kind::Url(url) => Ok(fetch_torrent(url, user_agent, deadline).await?.0),
        Kind::Metalink(path) => {
            let document = Metalink::read(path)?;
            Ok(resolve_metalink(&document, user_agent, deadline)
                .await?
                .meta)
        }
        Kind::MetalinkUrl(url) => {
            let document = fetch_metalink(url, user_agent, deadline).await?;
            Ok(resolve_metalink(&document, user_agent, deadline)
                .await?
                .meta)
        }
        _ => load_local(kind, env),
    }
}

/// [`resolve`] for a command that is synchronous, which is all of them.
///
/// The runtime is built **only when the source needs one**, so reading a local
/// `.torrent` costs exactly what it did before: no threads, no reactor. Every
/// caller runs before it has built a runtime of its own, and a caller that
/// changes gets an error naming the mistake rather than tokio's panic, which
/// is what `a_fetch_from_inside_a_runtime_is_an_error_not_a_panic` holds.
pub fn resolve_blocking(
    kind: &Kind,
    env: &mut Env,
    user_agent: &str,
    deadline: Duration,
    swarm: &SwarmResolve,
) -> Result<Metainfo> {
    if !kind.needs_network() {
        return load_local(kind, env);
    }
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err(Error::generic(
            "a source fetch was started from inside a runtime, and this is the blocking entry point",
        )
        .with("source_kind", kind.name()));
    }
    // A swarm needs a reactor with more than one thread on it: the session
    // listens, dials, announces and answers BEP 9 at the same time, and a
    // current-thread runtime is what every other command here avoids for
    // exactly that. A document fetch keeps the cheap one, because a `GET` is
    // one task.
    if matches!(kind, Kind::Magnet(_) | Kind::InfoHash(_)) {
        return crate::swarm::runtime()?.block_on(resolve(kind, env, user_agent, deadline, swarm));
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::generic(format!("cannot build a runtime to fetch the source: {e}")))?;
    runtime.block_on(resolve(kind, env, user_agent, deadline, swarm))
}

/// What resolving a magnet or a bare info hash is allowed to talk to.
///
/// Built from the "Resolving a magnet" flags every command that reads a source
/// carries. The default is a client's default: the DHT, local discovery and
/// the trackers the magnet itself names, which is the only way a magnet
/// carrying nothing but `xt` can be read at all. See `TODO/metainfo.md`,
/// T-241.
#[derive(Debug, Clone)]
pub struct SwarmResolve {
    /// Peers to ask before any are discovered.
    pub peers: Vec<std::net::SocketAddr>,
    pub enable_dht: bool,
    pub enable_lsd: bool,
    pub enable_trackers: bool,
}

impl Default for SwarmResolve {
    fn default() -> Self {
        Self {
            peers: Vec::new(),
            enable_dht: true,
            enable_lsd: true,
            enable_trackers: true,
        }
    }
}

impl SwarmResolve {
    /// The flags as the command line gave them.
    pub fn from_args(args: &crate::cli::SwarmSourceArgs) -> Result<Self> {
        Ok(Self {
            peers: crate::swarm::peer_addrs(&args.peers)?,
            enable_dht: !args.no_dht,
            enable_lsd: !args.no_lsd,
            enable_trackers: !args.no_tracker,
        })
    }
}

/// Join the swarm a magnet names and read the metadata off it.
///
/// Nothing is written and nothing is started: `list_only` stops the session at
/// the point the `info` dictionary has arrived, which is the same call
/// `download` makes before it applies `--exclude-file`. The bytes come back as
/// the session assembled them, so the info hash of what is parsed here equals
/// the one in the magnet.
///
/// The payload directory is a temporary one that is removed when this returns.
/// A session insists on having somewhere to write even when it is told not to
/// write, and the caller's working directory is not that place.
async fn resolve_from_swarm(
    source: &str,
    swarm: &SwarmResolve,
    deadline: Duration,
) -> Result<Metainfo> {
    use bit_cli_core::engine::{AddOptions, Engine, EngineOptions};

    let scratch = tempfile::tempdir()
        .map_err(|e| from_io(e, "cannot create a directory to resolve the metadata in"))?;
    let options = EngineOptions {
        download_directory: scratch.path().to_path_buf(),
        // The OS picks. A command that reads a torrent has no --port and
        // nothing about it wants a fixed one.
        listen_ports: 0..=0,
        enable_dht: swarm.enable_dht,
        enable_lsd: swarm.enable_lsd,
        enable_trackers: swarm.enable_trackers,
        ..Default::default()
    };
    let engine = Engine::start(&options).await?;
    let add = AddOptions {
        list_only: true,
        initial_peers: swarm.peers.clone(),
        disable_trackers: !swarm.enable_trackers,
        ..Default::default()
    };
    let resolved = tokio::time::timeout(deadline, engine.resolve_with(source, &add))
        .await
        .map_err(|_| {
            Error::timeout(format!(
                "{source}: the metadata did not resolve in {}ms",
                deadline.as_millis()
            ))
            .with("phase", "resolving_metadata")
            .with("source_kind", "magnet")
            .with(
                "waited_ms",
                deadline.as_millis().min(u128::from(u64::MAX)) as u64,
            )
        });
    let resolved = match resolved {
        Ok(inner) => inner,
        Err(timeout) => {
            engine.stop().await;
            return Err(timeout);
        }
    }?;
    let meta = Metainfo::parse(&resolved.torrent_bytes);
    engine.stop().await;
    meta
}

/// [`resolve_blocking`] with the two things the command line decides read out
/// of it, so a command resolves its source in one call.
///
/// `user_agent` is the command's own `--web-seed-user-agent` where it has one.
/// A caller that set an identity for its mirrors meant it for the document
/// those mirrors are described by too, and a command with no such flag gets
/// the same default every other HTTP request here uses.
pub fn resolve_source(
    kind: &Kind,
    env: &mut Env,
    global: &crate::cli::Global,
    user_agent: Option<&str>,
    swarm: &crate::cli::SwarmSourceArgs,
) -> Result<Metainfo> {
    let agent = user_agent
        .map(str::to_string)
        .unwrap_or_else(bit_cli_core::webseed::fetch::default_user_agent);
    let timeout = crate::swarm::optional_duration(&global.timeout, "timeout")?;
    let resolve = SwarmResolve::from_args(swarm)?;
    let budget = match kind {
        // A swarm lookup is not a fetch. Finding a peer, handshaking and
        // pulling the metadata takes longer than a `GET` of the same bytes,
        // and 30 seconds is a deadline a healthy magnet would miss.
        // `--timeout` still wins where the caller set one.
        Kind::Magnet(_) | Kind::InfoHash(_) => timeout.unwrap_or(RESOLVE_TIMEOUT),
        _ => deadline(timeout),
    };
    resolve_blocking(kind, env, &agent, budget, &resolve)
}

/// The deadline for one document fetch, from `--timeout` when it was given.
///
/// `--timeout` is the whole operation's deadline, and for a command that reads
/// a torrent and prints it the fetch **is** the operation, so the flag is the
/// deadline rather than a bound on it. Absent, [`FETCH_TIMEOUT`] applies.
pub fn deadline(timeout: Option<Duration>) -> Duration {
    timeout.unwrap_or(FETCH_TIMEOUT)
}

/// Largest `.torrent` accepted over HTTP, in bytes.
///
/// Sixteen mebibytes holds 838,860 SHA-1 piece hashes, which is past the
/// 262,104 pieces the read side caps a bitfield at, so this ceiling cannot be
/// what refuses a torrent this tree could otherwise handle. See
/// `TODO/peers.md`, T-195. It exists for the same reason
/// [`MAX_METALINK_BYTES`] does: the URL comes from the caller and the body
/// comes from whoever answers it.
const MAX_TORRENT_BYTES: usize = 16 * 1024 * 1024;

/// Fetch a `.torrent` over HTTP, keeping the bytes as well as the parse.
///
/// Both, because the caller hands the exact bytes to the session rather than
/// handing it the URL again. Fetching a URL twice can return two different
/// documents, and a run whose report describes one torrent while the session
/// downloads another is the worst kind of wrong answer.
pub async fn fetch_torrent(
    url: &str,
    user_agent: &str,
    deadline: Duration,
) -> Result<(Metainfo, Vec<u8>)> {
    let body = fetch_bytes(url, user_agent, MAX_TORRENT_BYTES, deadline).await?;
    let meta = Metainfo::parse(&body.bytes).map_err(|e| {
        // What arrived, not only where the parse gave up. A URL that serves a
        // directory listing fails on byte 0 being `<`, and "the server sent
        // text/html" is the sentence that tells a caller they pasted the page
        // rather than the file. See `TODO/cli-surface.md`, T-245.
        // `e` already begins "not a valid torrent", so this says what arrived
        // and lets the parser say what was wrong with it. Saying both in one
        // sentence made the message read "not a torrent: not a valid torrent".
        let error = Error::source_resolution(match &body.content_type {
            Some(kind) => format!("{url}: the server answered with {kind}: {e}"),
            None => format!("{url}: {e}"),
        })
        .with("url", url.to_string())
        .with("bytes", body.bytes.len());
        match &body.content_type {
            Some(kind) => error.with("content_type", kind.clone()),
            None => error,
        }
    })?;
    Ok((meta, body.bytes))
}

/// A fetched body and the one header that says what it is.
///
/// `Content-Type` is carried because a failure to parse is answered by what
/// arrived rather than by where the parse stopped.
struct Body {
    bytes: Vec<u8>,
    content_type: Option<String>,
}

/// Fetch a URL and return its body, failing on any status that is not success.
///
/// The body is read in chunks and stopped at `max_bytes`, so the cap bounds
/// what is held in memory rather than only what is returned. Reading it whole
/// and measuring it afterwards is what the Metalink path did until T-245, and
/// a server declaring a small `Content-Length` and sending more was read in
/// full before anything checked it.
async fn fetch_bytes(
    url: &str,
    user_agent: &str,
    max_bytes: usize,
    deadline: Duration,
) -> Result<Body> {
    let client = reqwest_client(user_agent, deadline)?;
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| fetch_error(e, url, "cannot fetch", deadline))?;
    let status = response.status();
    if !status.is_success() {
        return Err(Error::source_resolution(format!("{url}: {status}"))
            .with("url", url.to_string())
            .with("http_status", status.as_u16()));
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let mut body: Vec<u8> = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| fetch_error(e, url, "cannot read the body of", deadline))?
    {
        body.extend_from_slice(&chunk);
        if body.len() > max_bytes {
            return Err(Error::source_resolution(format!(
                "{url} answered with more than {max_bytes} bytes, which is larger than any document a source can be"
            ))
            .with("url", url.to_string())
            .with("max_bytes", max_bytes));
        }
    }
    Ok(Body {
        bytes: body,
        content_type,
    })
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
pub async fn fetch_metalink(url: &str, user_agent: &str, deadline: Duration) -> Result<Metalink> {
    let body = fetch_bytes(url, user_agent, MAX_METALINK_BYTES, deadline).await?;
    Metalink::parse(&body.bytes).map_err(|e| Error::source_resolution(format!("{url}: {e}")))
}

/// Read a Metalink and fetch the torrent it names.
///
/// The document may list several torrents, which is a mirror list for the
/// `.torrent` itself. They are tried in the document's own preferred order and
/// the first that parses wins; the failures are kept so the report can say the
/// preferred one was not the one used.
pub async fn resolve_metalink(
    document: &Metalink,
    user_agent: &str,
    deadline: Duration,
) -> Result<ResolvedMetalink> {
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
        match fetch_torrent(&mirror.url, user_agent, deadline).await {
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

/// Longest one fetch may take by default, connect and body together.
///
/// A tracker list, a `.torrent` or a Metalink that never finishes arriving
/// must not hold up the command that asked for it. A list fetch is always
/// bounded by this; a source fetch takes `--timeout` instead when the caller
/// set one, which is what [`deadline`] decides.
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

/// Longest a magnet or a bare info hash may take to resolve, by default.
///
/// The swarm counterpart of [`FETCH_TIMEOUT`], and larger than it because the
/// work is larger: find a peer through the DHT or a tracker, handshake it, and
/// pull the `info` dictionary over BEP 9. `--timeout` replaces it where the
/// caller set one, the same way it replaces the fetch deadline.
const RESOLVE_TIMEOUT: Duration = Duration::from_secs(60);

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
        .timeout(FETCH_TIMEOUT)
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

/// Turn a `reqwest` failure into an error that says whether the deadline ended
/// it.
///
/// A body that runs out of time comes back as "error decoding response body",
/// which is true of the transport and says nothing about the flag the caller
/// set. A run that gave up because its deadline fired exits 9 and names the
/// deadline, which is the number a caller can change. See
/// `TODO/cli-surface.md`, T-245.
fn fetch_error(err: reqwest::Error, url: &str, what: &str, deadline: Duration) -> Error {
    if err.is_timeout() {
        return Error::timeout(format!(
            "{url}: no answer within {}ms, which is what --timeout allows",
            deadline.as_millis()
        ))
        .with("url", url.to_string())
        .with(
            "timeout_ms",
            u64::try_from(deadline.as_millis()).unwrap_or(u64::MAX),
        );
    }
    Error::network(format!("{what} {url}: {err}"))
}

fn reqwest_client(user_agent: &str, deadline: Duration) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(deadline)
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

    /// T-246's third case. `classify` tested for `http://` and `https://` and
    /// fell through to "treat it as a path" for everything else, so a URL
    /// under any other scheme came back as a relative filename and the
    /// operating system described a name it could not parse.
    #[test]
    fn a_scheme_this_does_not_speak_is_named_as_a_scheme() {
        let env = env();
        for source in [
            "ftp://host/x.torrent",
            "FTP://HOST/x.torrent",
            "sftp://host/x",
            "file:///tmp/x.torrent",
            "ws://host/x",
        ] {
            let err = Kind::classify(source, &env).unwrap_err();
            assert_eq!(
                err.code(),
                bit_cli_core::ExitCode::Usage,
                "{source}: {}",
                err.message()
            );
            assert!(err.message().contains("is not a scheme"), "{source}");
            assert!(err.message().contains("http:// or https://"), "{source}");
            assert!(err.message().contains("magnet:"), "{source}");
            assert!(err.context().contains_key("scheme"), "{source}");
        }
    }

    /// The three that are spoken keep working, and a Windows path is not a
    /// URL under a scheme called `C`.
    #[test]
    fn the_schemes_that_are_spoken_still_classify() {
        let env = env();
        assert!(matches!(
            Kind::classify("http://host/x.torrent", &env),
            Ok(Kind::Url(_))
        ));
        assert!(matches!(
            Kind::classify("https://host/x.torrent", &env),
            Ok(Kind::Url(_))
        ));
        assert!(matches!(
            Kind::classify(&format!("magnet:?xt=urn:btih:{HEX}"), &env),
            Ok(Kind::Magnet(_))
        ));
        assert!(matches!(
            Kind::classify("https://host/x.meta4", &env),
            Ok(Kind::MetalinkUrl(_))
        ));
        assert!(matches!(
            Kind::classify("C://Users/me/x.torrent", &env),
            Ok(Kind::File(_))
        ));
    }

    /// T-246's first case. Opening a directory as a file is
    /// `ERROR_ACCESS_DENIED` on Windows and `EISDIR` on Unix, so the same
    /// input produced two different explanations and neither was true.
    #[test]
    fn a_directory_is_named_as_one_rather_than_as_a_permission_problem() {
        let temp = tempfile::tempdir().expect("temp dir");
        let err = read_torrent_file(temp.path()).unwrap_err();
        assert_eq!(err.code(), bit_cli_core::ExitCode::Usage);
        assert!(
            err.message().contains("is a directory"),
            "{}",
            err.message()
        );
        assert!(
            err.message().contains("bit-cli create"),
            "{}",
            err.message()
        );
        assert!(!err.context().contains_key("io_kind"), "{err:?}");
    }

    /// And the command a caller actually types reaches it.
    #[test]
    fn a_directory_as_a_source_exits_two_from_the_command_line() {
        use crate::test_support::{TorrentFixture, run_err};

        let fixture = TorrentFixture::multi_file();
        let directory = fixture.payload_dir();
        for command in ["info", "files", "tree", "verify", "magnet"] {
            let err = run_err(
                &[command, directory.to_str().expect("utf-8")],
                fixture.dir(),
                bit_cli_core::ExitCode::Usage,
            );
            assert!(err.contains("is a directory"), "{command}: {err}");
            assert!(err.contains("bit-cli create"), "{command}: {err}");
        }
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

    /// T-245's acceptance. Every one of these commands offers an HTTP URL in
    /// its `SOURCE` help and every one refused it, while `download` fetched
    /// the same URL and completed.
    ///
    /// Every field but two has to match the same torrent read off disk:
    /// "it worked" and "it reported the same torrent" are different claims,
    /// and only the second one is worth anything. The two are the timestamp,
    /// which is two runs, and `source_kind`, which differs because the source
    /// genuinely was a URL.
    ///
    /// The name carries no count on purpose. It said `four_commands_` until
    /// `tree` became the fifth, and a count in a test name is one more number
    /// two documents can disagree about. See `TODO/metainfo.md`, T-249.
    #[test]
    fn read_only_commands_resolve_a_torrent_over_http_and_report_what_the_file_reports() {
        use crate::test_support::{FileServer, TorrentFixture, run_json};

        let fixture = TorrentFixture::multi_file();
        // `verify` hashes the payload, so it has to be where the torrent says.
        fixture.place(&fixture.dir(), &[]);
        let server = FileServer::start(fixture.dir());
        let url = format!("{}album.torrent", server.base);

        for command in ["info", "files", "tree", "magnet", "verify"] {
            let local = run_json(&[command, fixture.path_str()], fixture.dir());
            let remote = run_json(&[command, &url], fixture.dir());
            let (local, remote) = (strip(local), strip(remote));
            assert_eq!(local, remote, "`bit-cli {command}` disagrees with itself");
        }
    }

    /// Everything two runs of the same command differ in for reasons that are
    /// not the source: when they ran, and what the source was.
    fn strip(mut doc: serde_json::Value) -> serde_json::Value {
        if let Some(object) = doc.as_object_mut() {
            object.remove("generated_at");
            object.remove("source_kind");
        }
        doc
    }

    /// The other half of T-245's acceptance: a URL that serves something else
    /// still fails, and says **what arrived** rather than only where the parse
    /// stopped. A caller who pasted a directory listing reads "text/html" and
    /// knows what they did; "unexpected byte `<`" needs them to know bencode.
    #[test]
    fn a_url_that_serves_a_page_fails_and_names_what_arrived() {
        use crate::test_support::{TorrentFixture, run_err};

        let fixture = TorrentFixture::multi_file();
        let port = answer_once(PAGE, "text/html; charset=utf-8");
        let url = format!("http://127.0.0.1:{port}/downloads/");

        let err = run_err(
            &["info", &url],
            fixture.dir(),
            bit_cli_core::ExitCode::SourceResolution,
        );
        assert!(err.contains(&url), "the URL is not in the message: {err}");
        assert!(
            err.contains("the server answered with text/html"),
            "the content type is not in the message: {err}"
        );
        assert!(err.contains("not a valid torrent"), "{err}");
    }

    /// A server with no `Content-Type` still fails, and says the one thing it
    /// can. The branch matters because plenty of object stores answer a
    /// `.torrent` with no type at all.
    #[test]
    fn a_body_that_is_not_a_torrent_fails_without_a_content_type_too() {
        use crate::test_support::{TorrentFixture, run_err};

        let fixture = TorrentFixture::multi_file();
        let port = answer_once(b"nope!", "");
        let url = format!("http://127.0.0.1:{port}/album.torrent");

        let err = run_err(
            &["info", &url],
            fixture.dir(),
            bit_cli_core::ExitCode::SourceResolution,
        );
        assert!(!err.contains("the server answered with"), "{err}");
        assert!(err.contains("not a valid torrent"), "{err}");
    }

    /// A directory listing, which is what a caller who pasted the wrong URL
    /// gets back. Its length is taken rather than written down, because a
    /// `Content-Length` that disagrees with the body is a hang rather than a
    /// failed assertion.
    const PAGE: &[u8] = b"<!doctype html><html><body>Index of /pub</body></html>";

    /// Bind loopback, answer the first request with `body`, and stop.
    ///
    /// Small enough to hold the whole exchange in the test that reads it,
    /// which `FileServer` is not: this needs to control the response headers,
    /// and `FileServer` exists to serve files. An empty `content_type` sends
    /// no such header at all, which is the other branch under test.
    fn answer_once(body: &'static [u8], content_type: &str) -> u16 {
        let listener =
            std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).expect("bind loopback");
        let port = listener.local_addr().expect("local addr").port();
        let mut head = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n",
            body.len()
        );
        if !content_type.is_empty() {
            head.push_str(&format!("Content-Type: {content_type}\r\n"));
        }
        head.push_str("\r\n");
        std::thread::spawn(move || {
            use std::io::{Read, Write};
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            // The request is read out before anything is written. A response
            // sent while the request is still arriving is a reset on Windows,
            // which reads as a network failure rather than as the body this
            // test is about.
            let mut request = Vec::new();
            let mut buf = [0u8; 1024];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => request.extend_from_slice(&buf[..n]),
                }
            }
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(body);
            let _ = stream.flush();
        });
        port
    }

    /// `resolve_blocking` is what a synchronous command calls, and every one of
    /// them runs before it has built a runtime of its own. A caller that
    /// changes gets an error naming the mistake rather than tokio's "cannot
    /// block the current thread from within a runtime" panic, which names
    /// nothing a reader of this tree can act on. T-245.
    #[tokio::test]
    async fn a_fetch_from_inside_a_runtime_is_an_error_not_a_panic() {
        let mut env = env();
        let kind = Kind::classify("https://e.com/x.torrent", &env).unwrap();
        let err = resolve_blocking(
            &kind,
            &mut env,
            "agent",
            Duration::from_secs(1),
            &SwarmResolve::default(),
        )
        .unwrap_err();
        assert!(
            err.message().contains("inside a runtime"),
            "{}",
            err.message()
        );
        assert_eq!(err.context()["source_kind"], "torrent_url");
    }

    /// And the guard sits after the local shortcut rather than before it, so a
    /// source that needs no fetch resolves from anywhere. `files --against`
    /// reads several local torrents and must not care what it is called from.
    #[tokio::test]
    async fn a_local_source_resolves_from_inside_a_runtime_because_it_never_fetches() {
        use crate::test_support::TorrentFixture;

        let fixture = TorrentFixture::multi_file();
        let mut env = Env::test(&[], fixture.dir()).0;
        let kind = Kind::classify(fixture.path_str(), &env).unwrap();
        let meta = resolve_blocking(
            &kind,
            &mut env,
            "agent",
            Duration::from_secs(1),
            &SwarmResolve::default(),
        )
        .unwrap();
        assert_eq!(meta.info_hash().hex(), fixture.info_hash);
    }

    /// A magnet from inside a runtime names the entry point rather than
    /// starting a swarm under one.
    ///
    /// **This asserted `load_local`'s refusal until 2026-08-25.** A magnet
    /// short-circuited to it before any runtime was considered, because no
    /// fetch was going to happen; now it needs a runtime like every other
    /// remote kind, so it reaches the same guard a URL does. The guard is
    /// worth keeping pointed at: building a second runtime inside one is a
    /// tokio panic, and this is the error that replaced it.
    #[tokio::test]
    async fn a_magnet_from_inside_a_runtime_is_an_error_not_a_panic() {
        let mut env = env();
        let kind = Kind::classify(&format!("magnet:?xt=urn:btih:{HEX}"), &env).unwrap();
        let err = resolve_blocking(
            &kind,
            &mut env,
            "agent",
            Duration::from_secs(1),
            &SwarmResolve::default(),
        )
        .unwrap_err();
        assert!(
            err.message().contains("blocking entry point"),
            "{}",
            err.message()
        );
    }

    /// The local-only path still refuses a magnet, and still says why.
    ///
    /// [`load_local`] is what a caller uses when it must not touch the
    /// network, and a magnet is the one kind it cannot answer. The message is
    /// the one it has always given; what changed is that `resolve` no longer
    /// forwards to it. See `TODO/metainfo.md`, T-241.
    #[test]
    fn the_local_only_path_still_refuses_a_magnet() {
        let mut env = env();
        let kind = Kind::classify(&format!("magnet:?xt=urn:btih:{HEX}"), &env).unwrap();
        let err = load_local(&kind, &mut env).unwrap_err();
        assert!(
            err.message().contains("no piece hashes"),
            "{}",
            err.message()
        );
    }

    /// A deadline that fires is exit 9 and says so. `reqwest` calls a body
    /// that ran out of time "error decoding response body", which is a
    /// statement about the transport and says nothing about the flag the
    /// caller set. T-245.
    ///
    /// Nothing here waits on a duration: the server accepts and never answers,
    /// so the request's own deadline is the only thing that can end it, and
    /// the assertion is on the code and the message rather than on the clock.
    #[test]
    fn a_fetch_that_runs_out_of_time_exits_nine_and_names_the_deadline() {
        let listener =
            std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).expect("bind loopback");
        let port = listener.local_addr().expect("local addr").port();
        std::thread::spawn(move || {
            // Held rather than dropped: a closed socket is a refused request,
            // which fails for a different reason and would prove nothing.
            let held = listener.accept();
            std::thread::sleep(std::time::Duration::from_secs(2));
            drop(held);
        });

        let mut env = env();
        let url = format!("http://127.0.0.1:{port}/x.torrent");
        let kind = Kind::classify(&url, &env).unwrap();
        let err = resolve_blocking(
            &kind,
            &mut env,
            "agent",
            Duration::from_millis(300),
            &SwarmResolve::default(),
        )
        .unwrap_err();
        assert_eq!(err.code(), bit_cli_core::ExitCode::Timeout);
        assert!(err.message().contains("--timeout"), "{}", err.message());
        assert_eq!(err.context()["timeout_ms"], 300);
    }

    /// `--timeout` is the operation's deadline rather than a ceiling on it, so
    /// a caller who allowed ten minutes gets ten minutes and one who allowed
    /// three seconds gets three.
    #[test]
    fn the_fetch_deadline_is_the_timeout_flag_when_one_was_given() {
        assert_eq!(deadline(None), FETCH_TIMEOUT);
        assert_eq!(
            deadline(Some(Duration::from_secs(3))),
            Duration::from_secs(3)
        );
        assert_eq!(
            deadline(Some(Duration::from_secs(600))),
            Duration::from_secs(600)
        );
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
