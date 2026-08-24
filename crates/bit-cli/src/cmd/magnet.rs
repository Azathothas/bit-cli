//! `bit-cli magnet`: convert a torrent to a magnet URI, or read one back.

use bit_cli_core::ExitCode;
use bit_cli_core::error::Result;
use bit_cli_core::torrent::Magnet;
use bit_cli_core::units::Size;
use serde::Serialize;

use crate::cli::{Global, SourceArgs};
use crate::env::Env;
use crate::output::{Renderer, field};
use crate::source::{Kind, resolve_source};

/// What `bit-cli magnet` reports.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub magnet: String,
    pub info_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<Size>,
    pub trackers: Vec<String>,
    pub web_seeds: Vec<String>,
    pub peers: Vec<String>,
    pub selected_files: Vec<u32>,
}

impl Report {
    fn from_magnet(magnet: &Magnet) -> Self {
        Self {
            magnet: magnet.to_uri(),
            info_hash: magnet.info_hash.map(|h| h.hex()).unwrap_or_default(),
            name: magnet.name.clone(),
            length: magnet.length.map(Size),
            trackers: magnet.trackers.clone(),
            web_seeds: magnet.web_seeds.clone(),
            peers: magnet.peers.clone(),
            selected_files: magnet.selected_files(),
        }
    }

    /// The text rendering.
    ///
    /// Converting a torrent prints only the URI, so `bit-cli magnet x.torrent`
    /// drops straight into another command with nothing to strip. Reading a
    /// magnet back prints the fields, because that is the question being asked.
    pub fn lines(&self, uri_only: bool) -> Vec<String> {
        if uri_only {
            return vec![self.magnet.clone()];
        }
        let mut out = vec![field("info hash", &self.info_hash)];
        if let Some(name) = &self.name {
            out.push(field("name", name));
        }
        if let Some(length) = self.length {
            out.push(field("size", length));
        }
        for tracker in &self.trackers {
            out.push(field("tracker", tracker));
        }
        for seed in &self.web_seeds {
            out.push(field("web seed", seed));
        }
        for peer in &self.peers {
            out.push(field("peer", peer));
        }
        if !self.selected_files.is_empty() {
            out.push(field(
                "selected files",
                format!("{:?}", self.selected_files),
            ));
        }
        out.push(field("magnet", &self.magnet));
        out
    }
}

/// Run the command.
pub fn run(
    args: &SourceArgs,
    global: &Global,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<ExitCode> {
    let kind = Kind::classify(&args.source, env)?;
    let (report, uri_only) = match &kind {
        Kind::Magnet(magnet) => (Report::from_magnet(magnet), false),
        _ => {
            let meta = resolve_source(&kind, env, global, None)?;
            (Report::from_magnet(&Magnet::from_metainfo(&meta)), true)
        }
    };
    renderer.emit(env, "magnet", &report, || report.lines(uri_only))?;
    Ok(ExitCode::Success)
}

#[cfg(test)]
mod tests {
    use crate::test_support::{TorrentFixture, run_json, run_ok};

    #[test]
    fn a_torrent_converts_to_a_magnet_and_nothing_else() {
        let fixture = TorrentFixture::multi_file();
        let out = run_ok(&["magnet", fixture.path_str()], fixture.dir());
        assert_eq!(out.lines().count(), 1, "output must be pipeable: {out}");
        assert!(out.starts_with("magnet:?xt=urn:btih:"), "{out}");
        assert!(out.contains(&fixture.info_hash), "{out}");
    }

    #[test]
    fn the_generated_magnet_carries_the_trackers_and_web_seeds() {
        let fixture = TorrentFixture::multi_file();
        let doc = run_json(&["magnet", fixture.path_str()], fixture.dir());
        assert_eq!(doc["info_hash"], fixture.info_hash);
        assert_eq!(doc["name"], "album");
        assert_eq!(doc["length"]["bytes"], 2000);
        assert_eq!(doc["trackers"][0], "udp://tracker.example.com:80");
        assert_eq!(doc["web_seeds"][0], "https://mirror.example.com/pub/");
    }

    #[test]
    fn a_magnet_reads_back_without_touching_the_network() {
        let fixture = TorrentFixture::multi_file();
        let uri = run_ok(&["magnet", fixture.path_str()], fixture.dir());
        let doc = run_json(&["magnet", uri.trim()], fixture.dir());
        assert_eq!(doc["info_hash"], fixture.info_hash);
        assert_eq!(doc["name"], "album");
    }

    #[test]
    fn the_round_trip_is_stable() {
        let fixture = TorrentFixture::multi_file();
        let first = run_ok(&["magnet", fixture.path_str()], fixture.dir());
        let doc = run_json(&["magnet", first.trim()], fixture.dir());
        assert_eq!(doc["magnet"].as_str().unwrap(), first.trim());
    }
}
