//! Command-line definitions.
//!
//! The target user is a script or an agent, not a person watching a terminal.
//! Two rules follow from that and they are not negotiable:
//!
//! - **stdout carries data only.** JSON, NDJSON, or the requested plain
//!   values. A caller doing `bit-cli ... --json | jq` must never see a log
//!   line in the pipe.
//! - **stderr carries logs, progress, warnings, and errors.**
//!
//! Short flags follow `aria2`. A letter `aria2` already assigns keeps its
//! meaning, and a letter it does not assign is only used where the meaning is
//! obvious. Reassigning an `aria2` letter to a different concept would let a
//! script written from muscle memory do something else silently, which is
//! worse than having no short flag at all. `docs/flags.md` holds the full
//! table and CI checks it.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Non-interactive BitTorrent and HTTP download tool.
#[derive(Debug, Parser)]
#[command(
    name = "bit-cli",
    version,
    about = "Fetch, create, verify, and seed torrents, with first-class web seed control.",
    long_about = None,
    disable_help_subcommand = true,
    args_conflicts_with_subcommands = false,
    subcommand_negates_reqs = true,
    // `clap` would give `--version` the short `-V`, which `aria2` assigns to
    // `--check-integrity`. Reassigning an `aria2` letter to a different
    // concept is what lets a script written from muscle memory do something
    // else silently, so `--version` has no short form here. See docs/flags.md.
    disable_version_flag = true,
)]
pub struct Cli {
    /// Print the version and exit.
    ///
    /// There is no short form: `-v` is verbosity and `-V` is
    /// `--check-integrity`, both following `aria2`.
    #[arg(long, action = clap::ArgAction::Version)]
    pub version: (),

    #[command(flatten)]
    pub global: Global,

    #[command(subcommand)]
    pub command: Option<Command>,

    /// Sources to download when no subcommand is given.
    ///
    /// `bit-cli <SOURCE>` is the same as `bit-cli download <SOURCE>`.
    #[arg(value_name = "SOURCE")]
    pub sources: Vec<String>,
}

/// Flags that apply to every subcommand.
#[derive(Debug, Args, Clone)]
#[command(next_help_heading = "Global options")]
pub struct Global {
    /// Emit machine-readable JSON on stdout. Implies --progress=none.
    #[arg(long, global = true)]
    pub json: bool,

    /// Emit newline-delimited JSON events on stdout as they happen.
    #[arg(long, global = true, conflicts_with = "json")]
    pub jsonl: bool,

    /// Print the output schema version and exit.
    #[arg(long, global = true)]
    pub schema_version: bool,

    /// Suppress all non-error output.
    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,

    /// Increase verbosity. Repeatable: -v, -vv, -vvv.
    ///
    /// `aria2` uses -v for --version, so `bit-cli` does not: --version has no
    /// short form here. See docs/flags.md.
    #[arg(short = 'v', long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Log level.
    #[arg(long, global = true, value_name = "LEVEL", default_value = "warn")]
    pub log_level: LogLevel,

    /// Log format.
    #[arg(long, global = true, value_name = "FMT", default_value = "text")]
    pub log_format: LogFormat,

    /// Append logs to a file. Rotates by size and count.
    #[arg(short = 'l', long, global = true, value_name = "PATH")]
    pub log_file: Option<PathBuf>,

    /// Rotate the log at this size.
    #[arg(long, global = true, value_name = "SIZE", default_value = "16MiB")]
    pub log_max_size: String,

    /// Keep this many rotated logs.
    #[arg(long, global = true, value_name = "N", default_value_t = 5)]
    pub log_max_files: u32,

    /// Enable detailed tracing for one subsystem without raising the global level.
    ///
    /// Repeatable or comma-separated. Subsystems: peer, handshake, tracker,
    /// dht, http, piece, picker, disk, ratelimit, retry, config.
    #[arg(long, global = true, value_name = "SUBSYSTEM", value_delimiter = ',')]
    pub trace: Vec<String>,

    /// Show credentials in trace output instead of redacting them.
    #[arg(long, global = true)]
    pub no_redact: bool,

    /// When to use colour. Honours NO_COLOR.
    #[arg(long, global = true, value_name = "WHEN", default_value = "auto")]
    pub color: ColorWhen,

    /// Progress rendering. Defaults to none when stdout is not a terminal.
    #[arg(long, global = true, value_name = "MODE", default_value = "auto")]
    pub progress: ProgressMode,

    /// Config file path.
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Ignore all config files.
    #[arg(long, global = true, conflicts_with = "config")]
    pub no_config: bool,

    /// Output directory.
    #[arg(short = 'd', long, global = true, value_name = "DIR")]
    pub dir: Option<PathBuf>,

    /// Resolve, validate, and report. Write nothing.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Overall operation deadline.
    #[arg(long, global = true, value_name = "DUR")]
    pub timeout: Option<String>,

    /// Stop after this long regardless of state.
    #[arg(long, global = true, value_name = "DUR")]
    pub stop_after: Option<String>,
}

/// Log severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    /// The `tracing` filter directive this level means.
    pub const fn directive(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }

    /// Raise the level by `steps`, which is what repeated `-v` does.
    pub fn raised(self, steps: u8) -> Self {
        let ladder = [
            Self::Off,
            Self::Error,
            Self::Warn,
            Self::Info,
            Self::Debug,
            Self::Trace,
        ];
        let current = ladder.iter().position(|l| *l == self).unwrap_or(2);
        ladder[(current + steps as usize).min(ladder.len() - 1)]
    }
}

/// Log rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum LogFormat {
    Text,
    Json,
}

/// When to colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum ColorWhen {
    Auto,
    Always,
    Never,
}

impl From<ColorWhen> for crate::env::ColorChoice {
    fn from(when: ColorWhen) -> Self {
        match when {
            ColorWhen::Auto => Self::Auto,
            ColorWhen::Always => Self::Always,
            ColorWhen::Never => Self::Never,
        }
    }
}

/// How progress is rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum ProgressMode {
    Auto,
    None,
    Plain,
    Json,
}

/// Composition mode for CLI-supplied web seeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
#[value(rename_all = "lower")]
pub enum WebSeedMode {
    #[default]
    Auto,
    Exact,
    Prefix,
    Template,
}

impl From<WebSeedMode> for bit_cli_core::webseed::Mode {
    fn from(mode: WebSeedMode) -> Self {
        match mode {
            WebSeedMode::Auto => Self::Auto,
            WebSeedMode::Exact => Self::Exact,
            WebSeedMode::Prefix => Self::Prefix,
            WebSeedMode::Template => Self::Template,
        }
    }
}

/// BEP 19 or BEP 17 wire style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
#[value(rename_all = "lower")]
pub enum WebSeedStyle {
    #[default]
    Auto,
    GetRight,
    Hoffman,
}

impl From<WebSeedStyle> for bit_cli_core::webseed::Style {
    fn from(style: WebSeedStyle) -> Self {
        match style {
            WebSeedStyle::Auto => Self::Auto,
            WebSeedStyle::GetRight => Self::GetRight,
            WebSeedStyle::Hoffman => Self::Hoffman,
        }
    }
}

/// The subcommands.
///
/// The variants differ a lot in size, which clippy notices. Boxing them would
/// mean a heap allocation and a deref at every match site to save stack on a
/// value that is parsed once and lives for the whole process. Not worth it.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Fetch to completion in the foreground, then exit.
    Download(DownloadArgs),

    /// Parse a torrent, magnet, or metalink and print its metadata.
    Info(SourceArgs),

    /// List files with index, path, size, and priority.
    Files(FilesArgs),

    /// Connect, sample the swarm, report peers, then exit.
    Peers(PeersArgs),

    /// Announce or scrape, report the result, then exit.
    Trackers(TrackersArgs),

    /// Inspect, validate, and read from HTTP sources.
    #[command(subcommand)]
    Webseed(WebseedCommand),

    /// Hash-check existing data against the torrent.
    Verify(VerifyArgs),

    /// Create a .torrent.
    Create(CreateArgs),

    /// Rewrite metainfo fields on an existing .torrent, writing a new file.
    Edit(EditArgs),

    /// Convert a torrent to a magnet URI, or resolve a magnet to metadata.
    Magnet(SourceArgs),

    /// Seed existing data in the foreground.
    Seed(SeedArgs),

    /// Measure a target.
    #[command(subcommand)]
    Bench(BenchCommand),

    /// Configuration.
    #[command(subcommand)]
    Config(ConfigCommand),

    /// Generate shell completions.
    Completions(CompletionsArgs),

    /// Generate a man page.
    Man(ManArgs),

    /// Version, build metadata, enabled features, and protocol support.
    Version,
}

/// A source and nothing else.
#[derive(Debug, Args)]
pub struct SourceArgs {
    /// A .torrent path, an HTTP(S) URL, a magnet URI, an info hash, a
    /// metalink, or `-` for stdin.
    #[arg(value_name = "SOURCE")]
    pub source: String,
}

/// Web seed flags, shared by every command that can attach one.
#[derive(Debug, Args, Clone, Default)]
#[command(next_help_heading = "Web seeds")]
pub struct WebSeedArgs {
    /// Source for the whole torrent, under the current composition mode.
    #[arg(long = "web-seed", value_name = "URL")]
    pub web_seed: Vec<String>,

    /// Shorthand for a source with composition=exact.
    #[arg(long = "web-seed-exact", value_name = "URL")]
    pub web_seed_exact: Vec<String>,

    /// Bind a scope selector to a source, as SELECTOR=URL.
    #[arg(long = "web-seed-for", value_name = "SEL=URL")]
    pub web_seed_for: Vec<String>,

    /// Composition mode for CLI-supplied sources.
    #[arg(long = "web-seed-mode", value_name = "MODE", default_value = "auto")]
    pub web_seed_mode: WebSeedMode,

    /// Template used when the mode is `template`.
    #[arg(long = "web-seed-template", value_name = "TMPL")]
    pub web_seed_template: Option<String>,

    /// Restrict CLI-supplied sources to these piece indices.
    #[arg(long = "web-seed-pieces", value_name = "RANGE")]
    pub web_seed_pieces: Option<String>,

    /// Restrict CLI-supplied sources to this byte range of the payload.
    #[arg(long = "web-seed-bytes", value_name = "RANGE")]
    pub web_seed_bytes: Option<String>,

    /// One URL per line. Blank lines and # comments are ignored.
    #[arg(long = "web-seed-file", value_name = "PATH")]
    pub web_seed_file: Vec<PathBuf>,

    /// Fetch a newline-separated URL list over HTTP.
    #[arg(long = "web-seed-list-url", value_name = "URL")]
    pub web_seed_list_url: Vec<String>,

    /// TOML or JSON binding table. Full control.
    #[arg(long = "web-seed-config", value_name = "PATH")]
    pub web_seed_config: Vec<PathBuf>,

    /// BEP 19 or BEP 17 wire style.
    #[arg(long = "web-seed-style", value_name = "STYLE", default_value = "auto")]
    pub web_seed_style: WebSeedStyle,

    /// Disable peers, DHT, PEX, LSD, and trackers. HTTP sources only.
    #[arg(long = "web-seed-only")]
    pub web_seed_only: bool,

    /// Ignore all web seeds, including the torrent's own url-list.
    #[arg(long = "no-web-seed", conflicts_with = "no_torrent_web_seed")]
    pub no_web_seed: bool,

    /// Ignore the torrent's url-list but keep CLI-supplied sources.
    #[arg(long = "no-torrent-web-seed")]
    pub no_torrent_web_seed: bool,

    /// Concurrent ranged requests per source.
    #[arg(long = "web-seed-concurrency", value_name = "N")]
    pub web_seed_concurrency: Option<usize>,

    /// Peer connections each source is presented over. Default: 1.
    ///
    /// One source is one peer to the torrent session, and a peer's blocks are
    /// written and verified one at a time on that connection's own task, so
    /// that path is what bounds the transfer. Several connections give the
    /// source several of them. `--web-seed-concurrency` is divided between
    /// them rather than multiplied by them, so this does not hit the mirror
    /// harder. Measured in TODO/webseed.md, T-009.
    #[arg(long = "web-seed-connections", value_name = "N")]
    pub web_seed_connections: Option<usize>,

    /// Concurrent ranged requests across all sources.
    #[arg(long = "web-seed-max-total", value_name = "N")]
    pub web_seed_max_total: Option<usize>,

    /// Bytes per ranged request. Independent of the torrent's piece length.
    #[arg(long = "web-seed-chunk-size", value_name = "SIZE")]
    pub web_seed_chunk_size: Option<String>,

    /// Per-request timeout.
    #[arg(long = "web-seed-timeout", value_name = "DUR")]
    pub web_seed_timeout: Option<String>,

    /// Connect timeout for web seed requests.
    #[arg(long = "web-seed-connect-timeout", value_name = "DUR")]
    pub web_seed_connect_timeout: Option<String>,

    /// Consecutive failed requests before a source is retired.
    ///
    /// A request that fails transiently after its own `--web-seed-retries`
    /// are spent drops the connection and reconnects, so a mirror that is
    /// down for a moment is not lost. This is how many of those in a row it
    /// takes before the source is out for the rest of the run. A success
    /// resets the count.
    #[arg(long = "web-seed-max-errors", value_name = "N")]
    pub web_seed_max_errors: Option<u32>,

    /// Reserved for a cooled-down source coming back. Sets the timer only.
    ///
    /// A source that runs out of its `--web-seed-max-errors` budget is
    /// retired for the rest of the run, so nothing waits this out today. See
    /// TODO/multi-source.md, T-137.
    #[arg(long = "web-seed-cooldown", value_name = "DUR")]
    pub web_seed_cooldown: Option<String>,

    /// Per-request retries before counting an error.
    #[arg(long = "web-seed-retries", value_name = "N")]
    pub web_seed_retries: Option<u32>,

    /// Statuses to retry that would otherwise retire the source.
    ///
    /// Codes and inclusive ranges: `403`, `403,429`, `500-599`. A CDN that
    /// signs URLs answers 403 when a signature expires and the next request
    /// to the stable URL is redirected to a fresh one, so `403` there is
    /// transient. `--web-seed-retries`, `--web-seed-max-errors`, and
    /// `--web-seed-cooldown` still bound it.
    #[arg(long = "web-seed-retry-status", value_name = "CODES")]
    pub web_seed_retry_status: Option<String>,

    /// Statuses that retire the source, which would otherwise be retried.
    ///
    /// The other direction of `--web-seed-retry-status`, same spelling. A
    /// code cannot be in both lists.
    #[arg(long = "web-seed-fatal-status", value_name = "CODES")]
    pub web_seed_fatal_status: Option<String>,

    /// User-Agent for web seed requests.
    #[arg(long = "web-seed-user-agent", value_name = "UA")]
    pub web_seed_user_agent: Option<String>,

    /// Extra header on web seed requests, as `Name: value`.
    #[arg(long = "web-seed-header", value_name = "K: V")]
    pub web_seed_header: Vec<String>,

    /// Credentials: basic:user:pass, bearer:TOKEN, netrc, or none.
    #[arg(long = "web-seed-auth", value_name = "SPEC")]
    pub web_seed_auth: Option<String>,

    /// Rate cap per source.
    #[arg(long = "web-seed-speed-limit", value_name = "RATE")]
    pub web_seed_speed_limit: Option<String>,

    /// When to hash-check HTTP-sourced data.
    #[arg(long = "web-seed-verify", value_name = "MODE", default_value = "piece")]
    pub web_seed_verify: VerifyWhen,

    /// Bias among sources. Higher wins when several can serve a piece.
    #[arg(long = "web-seed-priority", value_name = "N")]
    pub web_seed_priority: Option<i32>,

    /// Bias the picker toward HTTP when both a peer and a source have a piece.
    #[arg(long = "prefer-web-seed")]
    pub prefer_web_seed: bool,

    /// Fail the run if a declared source turns out to be unusable.
    #[arg(long = "web-seed-require")]
    pub web_seed_require: bool,
}

/// When HTTP-sourced data is hash-checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
#[value(rename_all = "lower")]
pub enum VerifyWhen {
    #[default]
    Piece,
    File,
    None,
}

/// `bit-cli download`.
#[derive(Debug, Args)]
pub struct DownloadArgs {
    /// Sources to fetch.
    #[arg(value_name = "SOURCE", required = true)]
    pub sources: Vec<String>,

    #[command(flatten)]
    pub web_seeds: WebSeedArgs,

    #[command(flatten)]
    pub trackers: TrackerArgs,

    #[command(flatten)]
    pub limits: LimitArgs,

    #[command(flatten)]
    pub selection: SelectionArgs,

    /// Listen port, or a range as START-END. `0` asks the OS for a free one.
    #[arg(long, value_name = "PORT")]
    pub port: Vec<String>,

    /// Try this peer before any are discovered, as HOST:PORT. Repeatable.
    ///
    /// A peer given here is dialled whether or not a tracker or the DHT ever
    /// answers, which is what makes a swarm of known members testable and a
    /// private one reachable without discovery.
    #[arg(long = "peer", value_name = "ADDR")]
    pub peers: Vec<String>,

    /// Disable the DHT.
    ///
    /// With `--peer` and `--no-tracker` this leaves a swarm of exactly the
    /// members named on the command line, which is what a measurement needs
    /// and what a private network wants.
    #[arg(long)]
    pub no_dht: bool,

    /// Disable local service discovery.
    #[arg(long)]
    pub no_lsd: bool,

    /// Sources fetched in parallel within this one invocation.
    #[arg(short = 'j', long, value_name = "N", default_value_t = 1)]
    pub max_concurrent_downloads: usize,

    /// Hash-check before starting.
    #[arg(short = 'V', long)]
    pub check_integrity: bool,

    /// Hash-check and exit.
    #[arg(long)]
    pub hash_check_only: bool,

    /// Resume a partial download. On by default.
    #[arg(short = 'c', long, overrides_with = "no_continue")]
    pub r#continue: bool,

    /// Refuse to write into a file that is already there.
    ///
    /// `--continue` is the default, so this is how a run says "these files
    /// should not exist yet". Without it a partial download resumes and a
    /// complete one is hash-checked and left alone. A flag that could only
    /// ever be on would not be a flag.
    #[arg(long = "no-continue", overrides_with = "continue")]
    pub no_continue: bool,

    /// Overwrite existing files.
    #[arg(long)]
    pub allow_overwrite: bool,

    /// Emit a progress event this often.
    #[arg(long, value_name = "DUR", default_value = "1s")]
    pub report_interval: String,
}

impl DownloadArgs {
    /// The arguments `bit-cli <SOURCE>` means, with every other flag left at
    /// its default.
    ///
    /// The bare form has to behave exactly like `bit-cli download <SOURCE>`,
    /// so it goes through the same argument type rather than a second path
    /// that could drift from it.
    pub fn from_sources(sources: Vec<String>) -> Self {
        Self {
            sources,
            web_seeds: WebSeedArgs::default(),
            trackers: TrackerArgs::default(),
            limits: LimitArgs::default(),
            selection: SelectionArgs::default(),
            port: Vec::new(),
            peers: Vec::new(),
            no_dht: false,
            no_lsd: false,
            max_concurrent_downloads: 1,
            check_integrity: false,
            hash_check_only: false,
            r#continue: true,
            no_continue: false,
            allow_overwrite: false,
            report_interval: "1s".to_string(),
        }
    }
}

/// Tracker flags.
#[derive(Debug, Args, Clone, Default)]
#[command(next_help_heading = "Trackers")]
pub struct TrackerArgs {
    /// Add a tracker at runtime. The .torrent is never rewritten.
    #[arg(long, value_name = "URL")]
    pub tracker: Vec<String>,

    /// One tracker per line. A blank line separates BEP 12 tiers.
    #[arg(long, value_name = "PATH")]
    pub tracker_file: Vec<PathBuf>,

    /// Fetch a tracker list over HTTP.
    #[arg(long, value_name = "URL")]
    pub tracker_list_url: Vec<String>,

    /// Remove trackers. `*` removes all.
    #[arg(long, value_name = "URL")]
    pub exclude_tracker: Vec<String>,

    /// Replace the torrent's tracker list instead of adding to it.
    #[arg(long)]
    pub replace_trackers: bool,

    /// Tracker request timeout.
    #[arg(long, value_name = "DUR")]
    pub tracker_timeout: Option<String>,

    /// Tracker connect timeout.
    #[arg(long, value_name = "DUR")]
    pub tracker_connect_timeout: Option<String>,

    /// Override the announce interval.
    #[arg(long, value_name = "DUR")]
    pub tracker_interval: Option<String>,

    /// Disable tracker announces entirely.
    #[arg(long)]
    pub no_tracker: bool,
}

/// Rate, peer, and lifecycle limits.
#[derive(Debug, Args, Clone, Default)]
#[command(next_help_heading = "Limits and lifecycle")]
pub struct LimitArgs {
    /// Download rate cap, per torrent.
    #[arg(long, value_name = "RATE")]
    pub max_download_rate: Option<String>,

    /// Upload rate cap, per torrent.
    #[arg(short = 'u', long, value_name = "RATE")]
    pub max_upload_rate: Option<String>,

    /// Download rate cap across the whole run.
    #[arg(long, value_name = "RATE")]
    pub max_overall_download_rate: Option<String>,

    /// Upload rate cap across the whole run.
    #[arg(long, value_name = "RATE")]
    pub max_overall_upload_rate: Option<String>,

    /// Peer connections per torrent.
    #[arg(long, value_name = "N")]
    pub max_peers: Option<usize>,

    /// Peer connections across the run.
    #[arg(long, value_name = "N")]
    pub max_peers_total: Option<usize>,

    /// Payload files kept open at once.
    ///
    /// Files open when they are first touched and the least recently opened
    /// closes when this cap is reached, so a torrent with twenty thousand
    /// files does not need twenty thousand descriptors.
    #[arg(long, value_name = "N", default_value_t = bit_cli_core::storage::DEFAULT_MAX_OPEN_FILES)]
    pub max_open_files: usize,

    /// Stop seeding at this ratio. 0 means do not seed.
    #[arg(long, value_name = "RATIO")]
    pub seed_ratio: Option<f64>,

    /// Stop seeding after this long.
    #[arg(long, value_name = "DUR")]
    pub seed_time: Option<String>,

    /// Give up if there is no progress for this long.
    #[arg(long, value_name = "DUR")]
    pub stop_timeout: Option<String>,

    /// Give up if the hash check has not finished in this long.
    ///
    /// Initialisation is reading the metadata and hash-checking whatever is
    /// on disk, and it is where a torrent can stop making progress without
    /// failing. The error names the phase and how far the check got, which a
    /// plain deadline does not.
    #[arg(long, value_name = "DUR", default_value = "10m")]
    pub init_timeout: String,

    /// Abort if the rate drops below this.
    #[arg(long, value_name = "RATE")]
    pub lowest_speed_limit: Option<String>,

    /// Run this command on success. Arguments arrive through the environment.
    #[arg(long, value_name = "COMMAND")]
    pub on_complete: Option<String>,

    /// Run this command on failure.
    #[arg(long, value_name = "COMMAND")]
    pub on_error: Option<String>,

    /// Run this command after every verified piece. High frequency.
    #[arg(long, value_name = "COMMAND")]
    pub on_piece_verified: Option<String>,
}

/// File selection and placement.
#[derive(Debug, Args, Clone, Default)]
#[command(next_help_heading = "File selection")]
pub struct SelectionArgs {
    /// Download only these files. Accepts ranges: 1-5,8,10-.
    #[arg(long, value_name = "INDEX", value_delimiter = ',')]
    pub select_file: Vec<String>,

    /// Skip these files.
    #[arg(long, value_name = "INDEX", value_delimiter = ',')]
    pub exclude_file: Vec<String>,

    /// Rename a file by index, as INDEX=PATH.
    #[arg(short = 'O', long, value_name = "INDEX=PATH")]
    pub index_out: Vec<String>,

    /// Write the payload here instead of using the torrent's name.
    #[arg(short = 'o', long, value_name = "PATH")]
    pub out: Option<PathBuf>,

    /// How disk space is allocated.
    #[arg(long, value_name = "METHOD", default_value = "sparse")]
    pub file_allocation: FileAllocation,

    /// Piece selection strategy.
    #[arg(long, value_name = "STRATEGY", default_value = "rarest-first")]
    pub piece_selector: PieceSelector,
}

/// How space is reserved for the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
#[value(rename_all = "lower")]
pub enum FileAllocation {
    None,
    Prealloc,
    #[default]
    Sparse,
    Falloc,
}

/// Which piece to ask for next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum PieceSelector {
    #[default]
    RarestFirst,
    Sequential,
    InOrder,
    Random,
}

/// `bit-cli files`.
#[derive(Debug, Args)]
pub struct FilesArgs {
    #[command(flatten)]
    pub source: SourceArgs,

    /// Sort key, as KEY or KEY:ORDER. Keys: index, path, size.
    #[arg(long, value_name = "KEY", default_value = "index")]
    pub sort: String,
}

/// `bit-cli peers`.
#[derive(Debug, Args)]
pub struct PeersArgs {
    #[command(flatten)]
    pub source: SourceArgs,

    /// How long to sample the swarm.
    #[arg(long, value_name = "DUR", default_value = "15s")]
    pub duration: String,

    /// Stop once this many distinct peers have been seen.
    #[arg(long, value_name = "N")]
    pub count: Option<usize>,

    /// Sort key, as KEY or KEY:ORDER. Keys: addr, client, speed, pieces.
    #[arg(long, value_name = "KEY", default_value = "addr")]
    pub sort: String,

    /// Listen port, or a range as START-END. `0` asks the OS for a free one.
    #[arg(long, value_name = "PORT")]
    pub port: Vec<String>,
}

/// `bit-cli trackers`.
#[derive(Debug, Args)]
pub struct TrackersArgs {
    #[command(flatten)]
    pub source: SourceArgs,

    #[command(flatten)]
    pub trackers: TrackerArgs,

    /// Scrape instead of announcing.
    #[arg(long)]
    pub scrape: bool,
}

/// `bit-cli webseed`.
#[derive(Debug, Subcommand)]
pub enum WebseedCommand {
    /// Resolve every binding and print the exact URL each file maps to. No network.
    List(WebseedListArgs),
    /// Probe each source: range support, size, redirects, TLS, latency.
    Test(WebseedTestArgs),
    /// Measure ranged-GET latency and throughput as concurrency scales.
    Probe(WebseedProbeArgs),
    /// Fetch one range from one source and verify it against the torrent.
    Fetch(WebseedFetchArgs),
}

/// `bit-cli webseed list`.
#[derive(Debug, Args)]
pub struct WebseedListArgs {
    #[command(flatten)]
    pub source: SourceArgs,

    #[command(flatten)]
    pub web_seeds: WebSeedArgs,
}

/// `bit-cli webseed test`.
#[derive(Debug, Args)]
pub struct WebseedTestArgs {
    #[command(flatten)]
    pub source: SourceArgs,

    #[command(flatten)]
    pub web_seeds: WebSeedArgs,

    /// Use HEAD rather than a one-byte ranged GET.
    #[arg(long)]
    pub head: bool,

    /// Sources probed at once.
    ///
    /// A real torrent can carry hundreds of web seeds: the Arch Linux ISO
    /// carries 468. Probing them one at a time takes minutes, and every probe
    /// is one request to a different host, so they do not contend.
    #[arg(long, value_name = "N", default_value_t = 16)]
    pub concurrency: usize,
}

/// `bit-cli webseed probe`.
#[derive(Debug, Args)]
pub struct WebseedProbeArgs {
    #[command(flatten)]
    pub source: SourceArgs,

    #[command(flatten)]
    pub web_seeds: WebSeedArgs,

    /// How long to run.
    #[arg(long, value_name = "DUR", default_value = "10s")]
    pub duration: String,

    /// Step concurrency and report the curve.
    #[arg(long, value_name = "SPEC", default_value = "1,2,4,8,16")]
    pub concurrency_sweep: String,
}

/// `bit-cli webseed fetch`.
#[derive(Debug, Args)]
pub struct WebseedFetchArgs {
    #[command(flatten)]
    pub source: SourceArgs,

    #[command(flatten)]
    pub web_seeds: WebSeedArgs,

    /// Fetch from exactly this URL.
    #[arg(long, value_name = "URL")]
    pub url: Option<String>,

    /// Fetch one piece.
    #[arg(long, value_name = "N", conflicts_with_all = ["pieces", "bytes"])]
    pub piece: Option<u32>,

    /// Fetch a piece range.
    #[arg(long, value_name = "RANGE", conflicts_with = "bytes")]
    pub pieces: Option<String>,

    /// Fetch a whole file by index.
    #[arg(long, value_name = "N")]
    pub file: Option<usize>,

    /// Fetch a byte range.
    #[arg(long, value_name = "RANGE")]
    pub bytes: Option<String>,

    /// Write the bytes here, or `-` for stdout. Writes nothing without this.
    #[arg(long, value_name = "PATH")]
    pub output: Option<String>,

    /// Verify against the torrent's piece hashes.
    #[arg(long, default_value_t = true)]
    pub verify: bool,
}

/// `bit-cli verify`.
#[derive(Debug, Args)]
pub struct VerifyArgs {
    #[command(flatten)]
    pub source: SourceArgs,

    /// Where the payload lives. Defaults to --dir.
    #[arg(long, value_name = "PATH")]
    pub data: Option<PathBuf>,

    /// Report the result of every piece, not just the failures.
    #[arg(long)]
    pub per_piece: bool,
}

/// `bit-cli create`.
#[derive(Debug, Args)]
pub struct CreateArgs {
    /// File or directory to build a torrent from.
    #[arg(value_name = "PATH")]
    pub path: PathBuf,

    /// Write here, or `-` for stdout. Defaults to alongside the input.
    #[arg(short = 'o', long, value_name = "TARGET")]
    pub output: Option<String>,

    /// Torrent name. Defaults to the input filename.
    #[arg(long, value_name = "TEXT")]
    pub name: Option<String>,

    /// Piece length. Accepts binary units. Chosen by heuristic when absent.
    #[arg(long, value_name = "SIZE")]
    pub piece_length: Option<String>,

    /// Metainfo version.
    #[arg(long, value_name = "V", default_value = "v1")]
    pub version: TorrentVersion,

    /// Primary tracker.
    #[arg(long, value_name = "URL")]
    pub announce: Option<String>,

    /// Add a BEP 12 tier. Repeatable. Comma-separates within a tier.
    #[arg(long, value_name = "URLS", value_delimiter = ',')]
    pub announce_tier: Vec<String>,

    /// Web seed written into `url-list` (BEP 19).
    #[arg(long, value_name = "URL")]
    pub web_seed: Vec<String>,

    /// HTTP seed written into `httpseeds` (BEP 17).
    #[arg(long, value_name = "URL")]
    pub http_seed: Vec<String>,

    /// DHT bootstrap node written into the torrent.
    #[arg(long, value_name = "HOST:PORT")]
    pub node: Vec<String>,

    /// Free-text comment.
    #[arg(long, value_name = "TEXT")]
    pub comment: Option<String>,

    /// The `source` key in the info dict. Changes the info hash.
    #[arg(long, value_name = "TEXT")]
    pub source: Option<String>,

    /// BEP 39 feed URL.
    #[arg(long, value_name = "URL")]
    pub update_url: Option<String>,

    /// Set the private flag (BEP 27).
    #[arg(long)]
    pub private: bool,

    /// Write per-file MD5 checksums. MD5 is not collision resistant.
    #[arg(long)]
    pub md5: bool,

    /// Include or, with a leading `!`, exclude paths.
    #[arg(long, value_name = "GLOB")]
    pub glob: Vec<String>,

    /// Respect .gitignore, .ignore, and .git/info/exclude.
    #[arg(long)]
    pub ignore: bool,

    /// Include hidden files.
    #[arg(long)]
    pub include_hidden: bool,

    /// Include junk files such as .DS_Store and Thumbs.db.
    #[arg(long)]
    pub include_junk: bool,

    /// Follow symlinks.
    #[arg(long)]
    pub follow_symlinks: bool,

    /// Deterministic file ordering, as KEY:ORDER.
    #[arg(long, value_name = "KEY:ORDER", default_value = "path:asc")]
    pub sort_by: String,

    /// Omit the `created by` field.
    #[arg(long)]
    pub no_created_by: bool,

    /// Omit the creation date. Required for byte-reproducible output.
    #[arg(long)]
    pub no_creation_date: bool,

    /// Permit a lint that would otherwise refuse the build. Repeatable.
    #[arg(long, value_name = "LINT")]
    pub allow: Vec<String>,

    /// Overwrite an existing output file.
    #[arg(long)]
    pub force: bool,

    /// Print the magnet URI to stdout.
    #[arg(long)]
    pub link: bool,

    /// Print a summary of what was created.
    #[arg(long)]
    pub show: bool,
}

/// Metainfo version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum TorrentVersion {
    V1,
    V2,
    Hybrid,
}

/// `bit-cli edit`.
#[derive(Debug, Args)]
pub struct EditArgs {
    /// The torrent to read.
    #[arg(value_name = "TORRENT")]
    pub torrent: PathBuf,

    /// Write here, or `-` for stdout. Never edits in place.
    #[arg(short = 'o', long, value_name = "TARGET")]
    pub output: Option<String>,

    /// Replace the primary tracker.
    #[arg(long, value_name = "URL")]
    pub announce: Option<String>,

    /// Add a BEP 12 tier. Repeatable.
    #[arg(long, value_name = "URLS", value_delimiter = ',')]
    pub announce_tier: Vec<String>,

    /// Drop every tracker.
    #[arg(long)]
    pub no_announce: bool,

    /// Add a web seed to `url-list`.
    #[arg(long, value_name = "URL")]
    pub web_seed: Vec<String>,

    /// Replace `url-list` rather than adding to it.
    #[arg(long)]
    pub replace_web_seeds: bool,

    /// Drop every web seed.
    #[arg(long, conflicts_with = "web_seed")]
    pub no_web_seed: bool,

    /// Add an HTTP seed to `httpseeds`.
    #[arg(long, value_name = "URL")]
    pub http_seed: Vec<String>,

    /// Replace the comment.
    #[arg(long, value_name = "TEXT")]
    pub comment: Option<String>,

    /// Drop the comment.
    #[arg(long, conflicts_with = "comment")]
    pub no_comment: bool,

    /// Replace the `created by` field.
    #[arg(long, value_name = "TEXT")]
    pub created_by: Option<String>,

    /// Drop the creation date.
    #[arg(long)]
    pub no_creation_date: bool,

    /// Add a DHT bootstrap node.
    #[arg(long, value_name = "HOST:PORT")]
    pub node: Vec<String>,

    /// Replace the BEP 39 feed URL.
    #[arg(long, value_name = "URL")]
    pub update_url: Option<String>,

    /// Permit an edit that changes the info hash.
    #[arg(long)]
    pub allow_new_infohash: bool,

    /// Overwrite an existing output file.
    #[arg(long)]
    pub force: bool,
}

/// `bit-cli seed`.
#[derive(Debug, Args)]
pub struct SeedArgs {
    #[command(flatten)]
    pub source: SourceArgs,

    #[command(flatten)]
    pub trackers: TrackerArgs,

    #[command(flatten)]
    pub limits: LimitArgs,

    /// Where the payload already lives. Defaults to --dir.
    #[arg(long, value_name = "PATH")]
    pub data: Option<PathBuf>,

    /// Hash-check before announcing.
    ///
    /// `full` is what happens today whatever this says: the session hash-checks
    /// the whole payload on add and offers no way to skip it. `quick` and
    /// `none` are accepted, warn, and do the same thing. See `TODO/disk-io.md`,
    /// T-016.
    #[arg(long, value_name = "MODE", default_value = "full")]
    pub verify: SeedVerify,

    /// BEP 16 superseeding for initial distribution.
    #[arg(long)]
    pub superseed: bool,

    /// Announce, report the tracker response, do not serve.
    #[arg(long)]
    pub announce_only: bool,

    /// Listen port, or a range as START-END.
    #[arg(long, value_name = "PORT")]
    pub port: Vec<String>,

    /// Disable the DHT.
    #[arg(long)]
    pub no_dht: bool,

    /// Disable peer exchange.
    #[arg(long)]
    pub no_pex: bool,

    /// Disable local service discovery.
    #[arg(long)]
    pub no_lsd: bool,

    /// Emit a progress event this often.
    #[arg(long, value_name = "DUR", default_value = "5s")]
    pub report_interval: String,

    /// Exit after this long with no connected peers.
    #[arg(long, value_name = "DUR")]
    pub exit_when_idle: Option<String>,
}

/// How much to hash-check before seeding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum SeedVerify {
    Full,
    Quick,
    None,
}

/// `bit-cli bench`.
///
/// Same size spread as [`Command`], and the same reasoning: parsed once, lives
/// for the run, not worth a box.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub enum BenchCommand {
    /// Download from a target and measure.
    Leech(BenchLeechArgs),
    /// Seed and measure what the swarm pulls.
    Seed(BenchArgs),
    /// Measure HTTP sources: latency percentiles, concurrency scaling, ranges.
    Webseed(BenchWebseedArgs),
    /// Measure the payload file under several writers, with no session.
    Disk(BenchDiskArgs),
    /// Synthetic peer load against a target.
    Swarm(BenchArgs),
    /// One-shot capability and reachability probe.
    Probe(BenchArgs),
}

/// Flags shared by every `bench` subcommand.
///
/// The report goes to stdout unless `--report <PATH>` names a file, in which
/// case stdout carries the text summary instead. `--format` decides how the
/// report is written; `--json` and `--jsonl` set it to `json` and `ndjson`.
#[derive(Debug, Args, Clone)]
#[command(next_help_heading = "Benchmark options")]
pub struct BenchShared {
    /// How long to run.
    #[arg(long, value_name = "DUR", default_value = "30s")]
    pub duration: String,

    /// Discard measurements from this initial window.
    #[arg(long, value_name = "DUR", default_value = "3s")]
    pub warmup: String,

    /// Time series resolution.
    #[arg(long, value_name = "DUR", default_value = "1s")]
    pub metrics_interval: String,

    /// Drive toward this rate rather than running flat out.
    #[arg(long, value_name = "RATE")]
    pub target_rate: Option<String>,

    /// Fixed concurrency.
    #[arg(long, value_name = "N", default_value_t = 8)]
    pub concurrency: usize,

    /// Step concurrency and report the curve.
    #[arg(long, value_name = "SPEC")]
    pub concurrency_sweep: Option<String>,

    /// Cap generated payload on disk.
    #[arg(long, value_name = "SIZE", default_value = "8GiB")]
    pub disk_budget: String,

    /// Bytes per request. Defaults to the source's own chunk size.
    #[arg(long, value_name = "SIZE")]
    pub request_size: Option<String>,

    /// A rate to report the result as a share of, such as what curl reached
    /// against the same URL.
    #[arg(long, value_name = "RATE")]
    pub ceiling: Option<String>,

    #[command(flatten)]
    pub report: ReportArgs,
}

/// Where a `bench` report goes and what it is checked against.
///
/// These are separate from the rest of [`BenchShared`] because every
/// subcommand has them and not every subcommand has a duration or a
/// concurrency. A flag that a subcommand cannot honour does not appear on it.
#[derive(Debug, Args, Clone, Default)]
#[command(next_help_heading = "Report options")]
pub struct ReportArgs {
    /// Write the full report here, or `-` for stdout. Default: stdout.
    #[arg(long, value_name = "PATH")]
    pub report: Option<String>,

    /// Report format: json, ndjson, csv, or text. `csv` carries the time
    /// series only, because a report is nested and a table is not.
    #[arg(long, value_name = "FMT", default_value = "json")]
    pub format: ReportFormat,

    /// Compare against a previous report and print the delta.
    #[arg(long, value_name = "PATH")]
    pub baseline: Option<PathBuf>,

    /// Exit 14 if sustained throughput falls below this.
    #[arg(long, value_name = "RATE")]
    pub fail_under: Option<String>,
}

/// How a bench report is written.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum ReportFormat {
    #[default]
    Json,
    Ndjson,
    Csv,
    Text,
}

/// A generic `bench` target.
#[derive(Debug, Args)]
pub struct BenchArgs {
    #[command(flatten)]
    pub source: SourceArgs,

    #[command(flatten)]
    pub shared: BenchShared,

    /// Synthetic peer count, for `bench swarm`.
    #[arg(long, value_name = "N", default_value_t = 8)]
    pub peers: usize,

    /// Synthetic torrent count, for `bench swarm`.
    #[arg(long, value_name = "N", default_value_t = 1)]
    pub torrents: usize,

    /// Per-torrent synthetic payload size.
    #[arg(long, value_name = "SIZE", default_value = "256MiB")]
    pub payload_size: String,

    /// Synthetic piece size.
    #[arg(long, value_name = "SIZE", default_value = "1MiB")]
    pub piece_size: String,
}

/// `bit-cli bench leech`.
///
/// A download with the clock running, so it carries the same source, tracker,
/// and limit flags `download` does. The payload has to land somewhere real:
/// the point of the measurement is what a download costs, and one that never
/// writes is measuring something else.
#[derive(Debug, Args)]
pub struct BenchLeechArgs {
    #[command(flatten)]
    pub source: SourceArgs,

    #[command(flatten)]
    pub web_seeds: WebSeedArgs,

    #[command(flatten)]
    pub trackers: TrackerArgs,

    #[command(flatten)]
    pub limits: LimitArgs,

    #[command(flatten)]
    pub shared: BenchShared,

    /// Listen port, or a range as START-END. `0` asks the OS for a free one.
    #[arg(long, value_name = "PORT")]
    pub port: Vec<String>,

    /// Try this peer before any are discovered, as HOST:PORT. Repeatable.
    #[arg(long = "peer", value_name = "ADDR")]
    pub peers: Vec<String>,

    /// How disk space is allocated for the payload.
    #[arg(long, value_name = "METHOD", default_value = "sparse")]
    pub file_allocation: FileAllocation,

    /// Overwrite whatever is already in the output directory.
    ///
    /// A benchmark run twice against the same directory would otherwise find
    /// the payload already there, hash-check it, finish immediately, and
    /// report a rate that is the hash checker's rather than the network's.
    #[arg(long, default_value_t = true, overrides_with = "keep_existing")]
    pub allow_overwrite: bool,

    /// Keep what is already in the output directory and resume onto it.
    #[arg(long, overrides_with = "allow_overwrite")]
    pub keep_existing: bool,

    /// Stop once the torrent completes, rather than running out `--duration`.
    /// On by default.
    #[arg(long, default_value_t = true, overrides_with = "run_full_duration")]
    pub stop_on_complete: bool,

    /// Keep running until `--duration` elapses even after the payload is in.
    #[arg(long, overrides_with = "stop_on_complete")]
    pub run_full_duration: bool,
}

/// `bit-cli bench webseed`.
#[derive(Debug, Args)]
pub struct BenchWebseedArgs {
    #[command(flatten)]
    pub source: SourceArgs,

    #[command(flatten)]
    pub web_seeds: WebSeedArgs,

    #[command(flatten)]
    pub shared: BenchShared,
}

/// `bit-cli bench disk`.
///
/// No torrent and no session: the same storage a download writes through,
/// driven straight from N threads. It takes only the shared flags it can
/// honour, because a fixed number of bytes has no warmup window and no target
/// rate. See `TODO/disk-io.md`, T-017.
#[derive(Debug, Args)]
pub struct BenchDiskArgs {
    #[command(flatten)]
    pub report: ReportArgs,

    /// Where the payload is written. Defaults to a directory this run makes
    /// under the system temporary directory and removes afterwards.
    #[arg(long, value_name = "DIR")]
    pub dir: Option<PathBuf>,

    /// Total bytes written per step.
    #[arg(long, value_name = "SIZE", default_value = "1GiB")]
    pub payload_size: String,

    /// Bytes per positioned write. The peer protocol's block is 16 KiB.
    #[arg(long, value_name = "SIZE", default_value = "16KiB")]
    pub block_size: String,

    /// How many threads write at once.
    #[arg(long, value_name = "N", default_value_t = 8)]
    pub concurrency: usize,

    /// Step the thread count and report the curve, for example `1,2,4,8`.
    #[arg(long, value_name = "SPEC")]
    pub concurrency_sweep: Option<String>,

    /// How the payload is spread over files. `shared` is one file with every
    /// thread interleaving into it, which is what a download does. `split`
    /// gives each thread its own file, which is the control.
    #[arg(long, value_name = "LAYOUT", default_value = "shared")]
    pub layout: DiskLayout,

    /// How disk space is allocated for the payload.
    #[arg(long, value_name = "METHOD", default_value = "sparse")]
    pub file_allocation: FileAllocation,

    /// How many payload files stay open at once. 0 uses the storage default.
    #[arg(long, value_name = "N", default_value_t = 0)]
    pub max_open_files: usize,

    /// Time series resolution.
    #[arg(long, value_name = "DUR", default_value = "1s")]
    pub metrics_interval: String,

    /// Stop a step once this much wall time has passed.
    #[arg(long, value_name = "DUR", default_value = "300s")]
    pub duration: String,

    /// Skip the read-back that checks every block landed where it was sent.
    #[arg(long)]
    pub no_verify: bool,
}

/// How `bench disk` spreads the payload over files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
#[value(rename_all = "lower")]
pub enum DiskLayout {
    /// One file, every thread interleaving blocks into it.
    #[default]
    Shared,
    /// One file per thread, each writing only its own.
    Split,
    /// One file opened once per thread, each writing through its own handle.
    Handles,
}

impl From<DiskLayout> for bit_cli_core::bench::disk::Layout {
    fn from(layout: DiskLayout) -> Self {
        match layout {
            DiskLayout::Shared => Self::Shared,
            DiskLayout::Split => Self::Split,
            DiskLayout::Handles => Self::Handles,
        }
    }
}

/// `bit-cli config`.
#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print the fully resolved configuration with the origin of every value.
    Show,
}

/// `bit-cli completions`.
#[derive(Debug, Args)]
pub struct CompletionsArgs {
    /// Which shell to generate for.
    #[arg(value_name = "SHELL")]
    pub shell: Shell,
}

/// Shells completions can be generated for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    Powershell,
    Elvish,
    Nushell,
}

/// `bit-cli man`.
#[derive(Debug, Args)]
pub struct ManArgs {
    /// Write the man page here instead of to stdout.
    #[arg(short = 'o', long, value_name = "PATH")]
    pub output: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn the_command_definition_is_internally_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn a_bare_source_is_a_download() {
        let cli = Cli::try_parse_from(["bit-cli", "a.torrent"]).unwrap();
        assert!(cli.command.is_none());
        assert_eq!(cli.sources, ["a.torrent"]);
    }

    #[test]
    fn the_download_subcommand_takes_the_same_source() {
        let cli = Cli::try_parse_from(["bit-cli", "download", "a.torrent"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Download(_))));
    }

    #[test]
    fn global_flags_work_before_and_after_the_subcommand() {
        for args in [
            ["bit-cli", "--json", "info", "a.torrent"],
            ["bit-cli", "info", "--json", "a.torrent"],
        ] {
            let cli = Cli::try_parse_from(args).unwrap();
            assert!(cli.global.json, "{args:?}");
        }
    }

    #[test]
    fn json_and_jsonl_cannot_both_be_asked_for() {
        assert!(
            Cli::try_parse_from(["bit-cli", "--json", "--jsonl", "info", "a.torrent"]).is_err()
        );
    }

    #[test]
    fn short_flags_keep_their_aria2_meanings() {
        let cli = Cli::try_parse_from([
            "bit-cli",
            "download",
            "-d",
            "/out",
            "-j",
            "4",
            "-V",
            "-c",
            "-u",
            "1MiB",
            "a.torrent",
        ])
        .unwrap();
        assert_eq!(
            cli.global.dir.as_deref(),
            Some(std::path::Path::new("/out"))
        );
        let Some(Command::Download(args)) = cli.command else {
            panic!("expected download")
        };
        assert_eq!(args.max_concurrent_downloads, 4);
        assert!(args.check_integrity);
        assert!(args.r#continue);
        assert_eq!(args.limits.max_upload_rate.as_deref(), Some("1MiB"));
    }

    #[test]
    fn v_is_verbosity_and_not_version() {
        let cli = Cli::try_parse_from(["bit-cli", "-vvv", "info", "a.torrent"]).unwrap();
        assert_eq!(cli.global.verbose, 3);
        // --version still works in its long form.
        let err = Cli::try_parse_from(["bit-cli", "--version"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }

    #[test]
    fn verbosity_raises_the_log_level_without_passing_the_top() {
        assert_eq!(LogLevel::Warn.raised(0), LogLevel::Warn);
        assert_eq!(LogLevel::Warn.raised(1), LogLevel::Info);
        assert_eq!(LogLevel::Warn.raised(2), LogLevel::Debug);
        assert_eq!(LogLevel::Warn.raised(3), LogLevel::Trace);
        assert_eq!(LogLevel::Warn.raised(99), LogLevel::Trace);
    }

    #[test]
    fn trace_subsystems_accept_commas_and_repetition() {
        let cli = Cli::try_parse_from([
            "bit-cli",
            "--trace",
            "http,piece",
            "--trace",
            "picker",
            "info",
            "a.torrent",
        ])
        .unwrap();
        assert_eq!(cli.global.trace, ["http", "piece", "picker"]);
    }

    #[test]
    fn every_web_seed_flag_parses() {
        let cli = Cli::try_parse_from([
            "bit-cli",
            "download",
            "--web-seed",
            "https://a.example.com/pub/",
            "--web-seed-exact",
            "https://cdn.example.com/blob",
            "--web-seed-for",
            "piece:0-511=https://b.example.com/",
            "--web-seed-mode",
            "prefix",
            "--web-seed-chunk-size",
            "4MiB",
            "--web-seed-header",
            "X-Region: apac",
            "--web-seed-auth",
            "bearer:tok",
            "--web-seed-only",
            "--web-seed-require",
            "a.torrent",
        ])
        .unwrap();
        let Some(Command::Download(args)) = cli.command else {
            panic!("expected download")
        };
        let ws = &args.web_seeds;
        assert_eq!(ws.web_seed, ["https://a.example.com/pub/"]);
        assert_eq!(ws.web_seed_exact, ["https://cdn.example.com/blob"]);
        assert_eq!(ws.web_seed_for, ["piece:0-511=https://b.example.com/"]);
        assert_eq!(ws.web_seed_mode, WebSeedMode::Prefix);
        assert_eq!(ws.web_seed_chunk_size.as_deref(), Some("4MiB"));
        assert_eq!(ws.web_seed_header, ["X-Region: apac"]);
        assert!(ws.web_seed_only);
        assert!(ws.web_seed_require);
    }

    #[test]
    fn no_web_seed_and_no_torrent_web_seed_are_mutually_exclusive() {
        assert!(
            Cli::try_parse_from([
                "bit-cli",
                "download",
                "--no-web-seed",
                "--no-torrent-web-seed",
                "a.torrent"
            ])
            .is_err()
        );
    }

    #[test]
    fn webseed_fetch_refuses_conflicting_range_selectors() {
        assert!(
            Cli::try_parse_from([
                "bit-cli",
                "webseed",
                "fetch",
                "--piece",
                "1",
                "--bytes",
                "0-100",
                "a.torrent"
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from(["bit-cli", "webseed", "fetch", "--piece", "1", "a.torrent"])
                .is_ok()
        );
    }

    #[test]
    fn create_takes_the_full_metainfo_surface() {
        let cli = Cli::try_parse_from([
            "bit-cli",
            "create",
            "--announce",
            "udp://a:80",
            "--announce-tier",
            "udp://b:80,udp://c:80",
            "--web-seed",
            "https://e.com/pub/",
            "--piece-length",
            "1MiB",
            "--private",
            "--no-creation-date",
            "--allow",
            "empty-file",
            "--sort-by",
            "path:asc",
            "./payload",
        ])
        .unwrap();
        let Some(Command::Create(args)) = cli.command else {
            panic!("expected create")
        };
        assert_eq!(args.announce.as_deref(), Some("udp://a:80"));
        assert_eq!(args.announce_tier, ["udp://b:80", "udp://c:80"]);
        assert_eq!(args.piece_length.as_deref(), Some("1MiB"));
        assert!(args.private);
        assert!(args.no_creation_date);
        assert_eq!(args.allow, ["empty-file"]);
    }

    #[test]
    fn every_subcommand_has_help_text() {
        for sub in Cli::command().get_subcommands() {
            assert!(
                sub.get_about().is_some(),
                "`{}` has no help text",
                sub.get_name()
            );
        }
    }

    #[test]
    fn no_short_flag_is_defined_twice() {
        use std::collections::HashMap;
        let command = Cli::command();
        let mut seen: HashMap<char, Vec<String>> = HashMap::new();
        let mut collect = |cmd: &clap::Command, prefix: &str| {
            for arg in cmd.get_arguments() {
                if let Some(short) = arg.get_short() {
                    seen.entry(short)
                        .or_default()
                        .push(format!("{prefix}{}", arg.get_id()));
                }
            }
        };
        collect(&command, "");
        for sub in command.get_subcommands() {
            let mut local: HashMap<char, Vec<String>> = HashMap::new();
            for arg in sub.get_arguments() {
                if let Some(short) = arg.get_short() {
                    local
                        .entry(short)
                        .or_default()
                        .push(arg.get_id().to_string());
                }
            }
            for (short, ids) in local {
                assert_eq!(
                    ids.len(),
                    1,
                    "`-{short}` is defined twice in `{}`: {ids:?}",
                    sub.get_name()
                );
            }
        }
    }

    #[test]
    fn short_flags_never_contradict_aria2() {
        // Letters aria2 assigns, and the `bit-cli` flag names that mean the
        // same concept. A short flag carrying one of these letters must name
        // one of the listed ids or not exist at all. Several names appear
        // where `bit-cli` spells the same concept differently in different
        // subcommands (`--out` for a payload, `--output` for a file it writes).
        let aria2: &[(char, &[&str])] = &[
            ('d', &["dir"]),
            ('o', &["out", "output"]),
            ('j', &["max-concurrent-downloads"]),
            ('u', &["max-upload-rate"]),
            ('q', &["quiet"]),
            ('c', &["continue"]),
            ('V', &["check-integrity"]),
            ('O', &["index-out"]),
            ('l', &["log-file"]),
        ];
        let command = Cli::command();
        let mut found: Vec<(char, String)> = Vec::new();
        for arg in command.get_arguments() {
            if let Some(short) = arg.get_short() {
                found.push((short, arg.get_id().to_string()));
            }
        }
        for sub in command.get_subcommands() {
            for arg in sub.get_arguments() {
                if let Some(short) = arg.get_short() {
                    found.push((short, arg.get_id().to_string()));
                }
            }
        }
        for (short, id) in found {
            if let Some((_, accepted)) = aria2.iter().find(|(c, _)| *c == short) {
                let name = id.replace('_', "-");
                assert!(
                    accepted.contains(&name.as_str()),
                    "`-{short}` means {accepted:?} in aria2 but `{name}` here"
                );
            }
        }
    }

    #[test]
    fn version_has_no_short_form() {
        // `clap` would give it `-V`, which `aria2` assigns to
        // `--check-integrity`. Reassigning an `aria2` letter to a different
        // concept is exactly what lets a script do something else silently.
        assert!(
            Cli::try_parse_from(["bit-cli", "-V"]).is_err(),
            "-V at the top level must not be --version"
        );
        let cli = Cli::try_parse_from(["bit-cli", "download", "-V", "a.torrent"]).unwrap();
        let Some(Command::Download(args)) = cli.command else {
            panic!("expected download")
        };
        assert!(args.check_integrity, "-V has to keep its aria2 meaning");
    }

    #[test]
    fn every_short_flag_is_documented_in_the_flags_table() {
        // `docs/flags.md` is the table A3.2 requires, and a table nothing
        // checks drifts within a week. This is the check: a short flag with no
        // row fails here rather than being discovered by a user whose script
        // did the wrong thing.
        let table = include_str!("../../../docs/flags.md");
        let command = Cli::command();

        let mut shorts: Vec<(char, String)> = Vec::new();
        let mut collect = |cmd: &clap::Command| {
            for arg in cmd.get_arguments() {
                if let Some(short) = arg.get_short() {
                    shorts.push((
                        short,
                        arg.get_long().unwrap_or(arg.get_id().as_str()).to_string(),
                    ));
                }
            }
        };
        collect(&command);
        for sub in command.get_subcommands() {
            collect(sub);
            for nested in sub.get_subcommands() {
                collect(nested);
            }
        }

        assert!(
            !shorts.is_empty(),
            "no short flags found, which cannot be right"
        );
        for (short, long) in shorts {
            let row = format!("| `-{short}` | `--{long}` |");
            assert!(
                table.contains(&row),
                "docs/flags.md has no row for `-{short}`/`--{long}`; add one:\n{row}"
            );
        }
    }
}
