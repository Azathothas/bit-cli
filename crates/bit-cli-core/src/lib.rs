//! Core of `bit-cli`: the engine, the web seed addressing model, metainfo
//! handling, and everything the binary renders.
//!
//! The library is usable without the binary and does not depend on `clap`.
//! Nothing here reads a global, a terminal, or an environment variable on its
//! own; configuration is passed in explicitly, which is what makes the whole
//! surface drivable from a test.

pub mod config;
pub mod engine;
pub mod error;
pub mod exit;
pub mod layout;
pub mod span;
pub mod time;
pub mod torrent;
pub mod tracker;
pub mod units;
pub mod webseed;

pub use error::{Error, Result};
pub use exit::ExitCode;
pub use layout::Layout;

/// The version of this build.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
