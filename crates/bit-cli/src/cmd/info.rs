//! `bit-cli info`: parse a torrent and print its metadata.

use bit_cli_core::ExitCode;
use bit_cli_core::error::Result;
use bit_cli_core::time::Timestamp;
use bit_cli_core::torrent::Metainfo;
use bit_cli_core::units::{Size, format_size};
use serde::Serialize;

use crate::cli::{Global, SourceArgs};
use crate::env::Env;
use crate::output::{Renderer, field};
use crate::source::{Kind, load_local};

/// What `bit-cli info` reports.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub info_hash: String,
    pub name: String,
    pub source_kind: String,
    pub multi_file: bool,
    pub private: bool,
    pub total: Size,
    pub piece_length: Size,
    pub piece_count: u32,
    pub file_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<Timestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_version: Option<i64>,
    pub trackers: Vec<Vec<String>>,
    pub web_seeds: Vec<String>,
    pub http_seeds: Vec<String>,
    pub nodes: Vec<String>,
    pub magnet: String,
}

impl Report {
    /// Build from parsed metadata.
    pub fn new(meta: &Metainfo, source_kind: &str) -> Self {
        let info = meta.info();
        Self {
            info_hash: meta.info_hash().hex(),
            name: info.name.clone(),
            source_kind: source_kind.to_string(),
            multi_file: info.multi_file,
            private: info.private,
            total: Size(info.total_length()),
            piece_length: Size(u64::from(info.piece_length)),
            piece_count: meta.layout().piece_count(),
            file_count: info.files.len(),
            comment: meta.comment(),
            created_by: meta.created_by(),
            creation_date: meta.creation_date(),
            source_tag: info.source.clone(),
            update_url: meta.update_url(),
            meta_version: info.meta_version,
            trackers: meta.announce_tiers(),
            web_seeds: meta.url_list(),
            http_seeds: meta.http_seeds(),
            nodes: meta.nodes(),
            magnet: bit_cli_core::torrent::Magnet::from_metainfo(meta).to_uri(),
        }
    }

    /// The text rendering.
    pub fn lines(&self) -> Vec<String> {
        let mut out = vec![
            field("name", &self.name),
            field("info hash", &self.info_hash),
            field("size", format_size(self.total.0)),
            field("files", self.file_count),
            field(
                "pieces",
                format!(
                    "{} x {}",
                    self.piece_count,
                    format_size(self.piece_length.0)
                ),
            ),
            field("private", self.private),
        ];
        if let Some(comment) = &self.comment {
            out.push(field("comment", comment));
        }
        if let Some(created_by) = &self.created_by {
            out.push(field("created by", created_by));
        }
        if let Some(when) = self.creation_date {
            out.push(field("created", when.iso()));
        }
        if let Some(tag) = &self.source_tag {
            out.push(field("source", tag));
        }
        if let Some(url) = &self.update_url {
            out.push(field("update url", url));
        }
        for (index, tier) in self.trackers.iter().enumerate() {
            out.push(field(&format!("tracker tier {index}"), tier.join(", ")));
        }
        for seed in &self.web_seeds {
            out.push(field("web seed", seed));
        }
        for seed in &self.http_seeds {
            out.push(field("http seed", seed));
        }
        for node in &self.nodes {
            out.push(field("dht node", node));
        }
        out.push(field("magnet", &self.magnet));
        out
    }
}

/// Run the command.
pub fn run(
    args: &SourceArgs,
    _global: &Global,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<ExitCode> {
    let kind = Kind::classify(&args.source, env)?;
    let meta = load_local(&kind, env)?;
    let report = Report::new(&meta, kind.name());
    renderer.emit(env, "info", &report, || report.lines())?;
    Ok(ExitCode::Success)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TorrentFixture, run_ok};

    #[test]
    fn info_reports_the_torrent_in_text() {
        let fixture = TorrentFixture::multi_file();
        let out = run_ok(&["info", fixture.path_str()], fixture.dir());
        assert!(out.contains("album"), "{out}");
        assert!(out.contains(&fixture.info_hash), "{out}");
        assert!(out.contains("magnet:?xt=urn:btih:"), "{out}");
    }

    #[test]
    fn info_reports_the_same_facts_in_json() {
        let fixture = TorrentFixture::multi_file();
        let (env, captured) =
            crate::env::Env::test(&["info", "--json", fixture.path_str()], fixture.dir());
        let mut env = env;
        assert_eq!(crate::run(&mut env), ExitCode::Success);
        let doc = captured.json().unwrap();
        assert_eq!(doc["name"], "album");
        assert_eq!(doc["info_hash"], fixture.info_hash);
        assert_eq!(doc["file_count"], 2);
        assert_eq!(doc["total"]["bytes"], 2000);
        assert_eq!(doc["total"]["human"], "1.95 KiB");
        assert_eq!(doc["piece_length"]["bytes"], 1024);
        assert_eq!(doc["piece_count"], 2);
        assert_eq!(doc["schema_version"], crate::output::SCHEMA_VERSION);
    }

    #[test]
    fn every_number_in_the_text_output_is_also_a_json_field() {
        let fixture = TorrentFixture::multi_file();
        let text = run_ok(&["info", fixture.path_str()], fixture.dir());
        let (mut env, captured) =
            crate::env::Env::test(&["info", "--json", fixture.path_str()], fixture.dir());
        crate::run(&mut env);
        let doc = captured.json().unwrap();

        // Anything a person can read is a field a script can reach.
        assert!(text.contains(doc["name"].as_str().unwrap()));
        assert!(text.contains(doc["info_hash"].as_str().unwrap()));
        assert!(text.contains(doc["total"]["human"].as_str().unwrap()));
        assert!(text.contains(&doc["file_count"].to_string()));
        assert!(text.contains(&doc["piece_count"].to_string()));
    }

    #[test]
    fn a_missing_torrent_exits_with_the_source_resolution_code() {
        let fixture = TorrentFixture::multi_file();
        let (mut env, captured) = crate::env::Env::test(&["info", "nope.torrent"], fixture.dir());
        assert_eq!(crate::run(&mut env), ExitCode::SourceResolution);
        assert_eq!(captured.out(), "");
        assert!(captured.err().contains("error:"));
    }

    #[test]
    fn a_magnet_says_it_needs_the_swarm_rather_than_guessing() {
        let fixture = TorrentFixture::multi_file();
        let magnet = format!("magnet:?xt=urn:btih:{}", fixture.info_hash);
        let (mut env, captured) = crate::env::Env::test(&["info", &magnet], fixture.dir());
        assert_eq!(crate::run(&mut env), ExitCode::SourceResolution);
        assert!(
            captured.err().contains("no piece hashes"),
            "{}",
            captured.err()
        );
    }
}
