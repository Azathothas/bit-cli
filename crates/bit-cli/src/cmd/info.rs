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
    /// What this torrent's own encoding did that a canonical encoder would
    /// not. Absent for a torrent encoded the way BEP 3 describes, which is
    /// almost all of them.
    ///
    /// `bit-cli` reads these rather than refusing them, because the `info`
    /// bytes are kept verbatim and never re-encoded, so the info hash is
    /// unaffected. A tool that **does** re-encode would produce a different
    /// hash from the same file, which is why this is reported rather than
    /// dropped. See `TODO/metainfo.md`, T-172.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<bit_cli_core::torrent::bencode::Encoding>,
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
            encoding: match meta.encoding().is_canonical() {
                true => None,
                false => Some(meta.encoding().clone()),
            },
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
        // Said in the text output as well as the JSON, because a caller
        // eyeballing a torrent before publishing it is exactly who needs to
        // know its encoding is not canonical.
        if let Some(encoding) = &self.encoding {
            for note in encoding.notes() {
                out.push(field("encoding", note));
            }
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

    /// A torrent written the way uTorrent/2210 wrote the one in intermodal
    /// issue 454: keys out of order, and a trailing newline for good measure.
    ///
    /// Every field is spelled out here rather than built by `create`, because
    /// `create` cannot produce this file: it is what this module means by
    /// canonical. See `TODO/metainfo.md`, T-172.
    fn sloppy_torrent() -> Vec<u8> {
        let info = {
            // Built rather than written as one literal: the piece hashes are
            // sixty bytes and a line continuation inside a byte string is one
            // more thing to get wrong in a fixture whose whole point is exact
            // bytes.
            let mut info = Vec::new();
            info.extend_from_slice(b"d12:piece lengthi1024e4:name9:movie.bin");
            info.extend_from_slice(b"6:lengthi3000e6:pieces60:");
            info.extend_from_slice(&[b'0'; 60]);
            info.push(b'e');
            info
        };
        let mut out = Vec::new();
        out.extend_from_slice(b"d4:info");
        out.extend_from_slice(&info);
        out.extend_from_slice(b"8:announce28:udp://tracker.example.com:80e");
        out.push(b'\n');
        out
    }

    /// The bytes above, read rather than refused, with the info hash taken
    /// over the `info` dictionary exactly as it was written.
    #[test]
    fn a_torrent_with_unsorted_keys_and_a_trailing_newline_is_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sloppy.torrent");
        let bytes = sloppy_torrent();
        std::fs::write(&path, &bytes).unwrap();

        let (env, captured) = crate::env::Env::test(
            &["info", "--json", path.to_str().unwrap()],
            dir.path().to_path_buf(),
        );
        let mut env = env;
        assert_eq!(
            crate::run(&mut env),
            ExitCode::Success,
            "it must not refuse"
        );
        let doc = captured.json().unwrap();
        assert_eq!(doc["name"], "movie.bin");
        assert_eq!(doc["total"]["bytes"], 3000);

        // The hash is over the original `info` bytes, not over a re-encoding
        // of them. That is the property that makes tolerance safe, and it is
        // why this torrent opens in `bit-cli` with the same info hash every
        // other client gives it.
        let start = bytes.windows(7).position(|w| w == b"d4:info").unwrap() + 7;
        let end = bytes.len() - "8:announce28:udp://tracker.example.com:80e\n".len();
        let expected: String = <sha1::Sha1 as sha1::Digest>::digest(&bytes[start..end])
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(doc["info_hash"], expected);

        // And it says so rather than accepting it silently.
        let encoding = &doc["encoding"];
        assert_eq!(encoding["unsorted_inside_info"], true, "{doc}");
        assert_eq!(encoding["unsorted_dicts"].as_array().unwrap().len(), 2);
        assert_eq!(encoding["trailing_bytes"], 1);
    }

    /// The text output says it too, because a caller eyeballing a torrent
    /// before publishing it is who needs to know.
    #[test]
    fn the_text_output_names_the_rule_that_was_bent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sloppy.torrent");
        std::fs::write(&path, sloppy_torrent()).unwrap();
        let out = run_ok(&["info", path.to_str().unwrap()], dir.path().to_path_buf());
        assert!(out.contains("BEP 3"), "{out}");
        assert!(out.contains("inside `info`"), "{out}");
        assert!(out.contains("whitespace or NUL"), "{out}");
    }

    /// An ordinary torrent reports nothing, so the field is not noise.
    #[test]
    fn a_canonical_torrent_reports_no_encoding_notes() {
        let fixture = TorrentFixture::multi_file();
        let (env, captured) =
            crate::env::Env::test(&["info", "--json", fixture.path_str()], fixture.dir());
        let mut env = env;
        assert_eq!(crate::run(&mut env), ExitCode::Success);
        let doc = captured.json().unwrap();
        assert!(doc.get("encoding").is_none(), "{doc}");
    }

    /// Bytes after the top-level dictionary that are not whitespace or NUL are
    /// still refused, and the message says what the rule is.
    #[test]
    fn junk_after_the_top_level_dictionary_is_still_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("junk.torrent");
        let mut bytes = sloppy_torrent();
        bytes.extend_from_slice(b"XYZ");
        std::fs::write(&path, &bytes).unwrap();
        let err = crate::test_support::run_err(
            &["info", path.to_str().unwrap()],
            dir.path().to_path_buf(),
            ExitCode::SourceResolution,
        );
        assert!(err.contains("whitespace and NUL"), "{err}");
        assert!(err.contains("top-level dictionary"), "{err}");
    }

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

    /// `TODO/metainfo.md` T-171. Both web seed keys are written as a bare
    /// bencoded string, and `info` has to report both rather than an empty
    /// list. Reading `url-list` for both shapes and `httpseeds` for one is how
    /// a torrent that names an HTTP seed reports none.
    #[test]
    fn a_web_seed_key_written_as_a_string_is_still_reported() {
        let fixture = TorrentFixture::web_seed_keys_as_strings();
        let doc =
            crate::test_support::run_json(&["info", "--json", fixture.path_str()], fixture.dir());
        assert_eq!(doc["info_hash"], fixture.info_hash);
        assert_eq!(doc["web_seeds"].as_array().unwrap().len(), 1);
        assert_eq!(doc["web_seeds"][0], "https://getright.example.com/pub/");
        assert_eq!(doc["http_seeds"].as_array().unwrap().len(), 1);
        assert_eq!(doc["http_seeds"][0], "https://hoffman.example.com/");

        let text = run_ok(&["info", fixture.path_str()], fixture.dir());
        assert!(text.contains("https://hoffman.example.com/"), "{text}");
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
