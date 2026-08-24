use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bencode::{self, Value};
use crate::xml;

#[derive(Debug, Clone)]
pub struct FeedItem {
    pub title: String,
    pub link: String,
    pub is_torrent: bool,
    pub guid: String,
}

#[derive(Debug, Clone)]
pub struct RssFeed {
    pub url: String,
    pub title: String,
    pub items: Vec<FeedItem>,
    pub last_poll: u64,
    pub poll_interval_secs: u64,
}

#[derive(Debug, Clone)]
pub struct RssRule {
    pub name: String,
    pub feed_url: String,
    pub pattern: String,
}

pub struct RssState {
    pub feeds: Vec<RssFeed>,
    pub rules: Vec<RssRule>,
    pub seen_guids: Vec<String>,
}

pub const MAX_SEEN_GUIDS: usize = 50_000;
const MAX_RSS_STATE_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const MAX_RSS_FEEDS: usize = 1_024;
pub(crate) const MAX_RSS_RULES: usize = 1_024;
pub(crate) const MAX_RSS_TEXT_BYTES: usize = 8 * 1024;
pub(crate) const MAX_RSS_PATTERN_BYTES: usize = 512;
const MAX_FEED_ITEMS: usize = 2_000;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

impl RssState {
    pub fn new() -> Self {
        Self {
            feeds: Vec::new(),
            rules: Vec::new(),
            seen_guids: Vec::new(),
        }
    }
}

pub fn parse_feed(data: &[u8]) -> Result<(String, Vec<FeedItem>), String> {
    let root = xml::parse(data).ok_or_else(|| "invalid xml".to_string())?;
    match root.local_name() {
        name if name.eq_ignore_ascii_case("rss") => parse_rss(&root),
        name if name.eq_ignore_ascii_case("feed") => parse_atom(&root),
        name if name.eq_ignore_ascii_case("rdf") => parse_rdf(&root),
        _ => Err(format!("unknown feed root tag: {}", root.tag)),
    }
}

fn parse_rss(root: &xml::XmlNode) -> Result<(String, Vec<FeedItem>), String> {
    let channel = root.child("channel").ok_or("missing <channel>")?;
    let title = channel
        .child("title")
        .map(|node| node.text.trim().to_string())
        .unwrap_or_default();
    Ok((title, parse_rss_items(channel.children_by_tag("item"))))
}

fn parse_rdf(root: &xml::XmlNode) -> Result<(String, Vec<FeedItem>), String> {
    let title = root
        .child("channel")
        .and_then(|channel| channel.child("title"))
        .map(|node| node.text.trim().to_string())
        .unwrap_or_default();
    Ok((title, parse_rss_items(root.children_by_tag("item"))))
}

fn parse_rss_items(item_nodes: Vec<&xml::XmlNode>) -> Vec<FeedItem> {
    let mut items = Vec::new();
    for item_node in item_nodes.into_iter().take(MAX_FEED_ITEMS) {
        let item_title = item_node
            .child("title")
            .map(|node| node.text.trim().to_string())
            .unwrap_or_default();
        let link = item_node
            .child("link")
            .map(|node| node.text.trim().to_string())
            .unwrap_or_default();
        let guid = item_node
            .child("guid")
            .map(|node| node.text.trim().to_string())
            .unwrap_or_default();
        let enclosure = item_node
            .children_by_tag("enclosure")
            .into_iter()
            .find_map(|node| {
                let url = node.attr("url")?.trim();
                let content_type = node.attr("type").unwrap_or("");
                (!url.is_empty() && (is_torrent_url(url) || is_torrent_content_type(content_type)))
                    .then(|| (url.to_string(), true))
            });
        let magnet = item_node
            .child("magnetURI")
            .map(|node| node.text.trim())
            .filter(|value| is_torrent_url(value))
            .map(|value| (value.to_string(), true));
        let link_is_torrent = is_torrent_url(&link);
        let (final_link, is_torrent) = enclosure.or(magnet).unwrap_or((link, link_is_torrent));
        let final_guid = if guid.is_empty() {
            final_link.clone()
        } else {
            guid
        };
        if final_link.trim().is_empty()
            || item_title.len() > MAX_RSS_TEXT_BYTES
            || final_link.len() > MAX_RSS_TEXT_BYTES
            || final_guid.len() > MAX_RSS_TEXT_BYTES
        {
            continue;
        }
        items.push(FeedItem {
            title: item_title,
            link: final_link,
            is_torrent,
            guid: final_guid,
        });
    }
    items
}

fn parse_atom(root: &xml::XmlNode) -> Result<(String, Vec<FeedItem>), String> {
    let title = root
        .child("title")
        .map(|node| node.text.trim().to_string())
        .unwrap_or_default();
    let mut items = Vec::new();
    for entry in root
        .children_by_tag("entry")
        .into_iter()
        .take(MAX_FEED_ITEMS)
    {
        let entry_title = entry
            .child("title")
            .map(|node| node.text.trim().to_string())
            .unwrap_or_default();
        let id = entry
            .child("id")
            .map(|node| node.text.trim().to_string())
            .unwrap_or_default();
        let links = entry.children_by_tag("link");
        let preferred = links.iter().find_map(|node| {
            let href = node.attr("href")?.trim();
            let rel = node.attr("rel").unwrap_or("");
            let content_type = node.attr("type").unwrap_or("");
            (!href.is_empty()
                && rel.eq_ignore_ascii_case("enclosure")
                && (is_torrent_url(href) || is_torrent_content_type(content_type)))
            .then(|| (href.to_string(), true))
        });
        let torrent_link = links.iter().find_map(|node| {
            let href = node.attr("href")?.trim();
            is_torrent_url(href).then(|| (href.to_string(), true))
        });
        let fallback = links.iter().find_map(|node| {
            let href = node.attr("href")?.trim();
            let rel = node.attr("rel").unwrap_or("alternate");
            (!href.is_empty() && rel.eq_ignore_ascii_case("alternate"))
                .then(|| (href.to_string(), is_torrent_url(href)))
        });
        let (link, is_torrent) = preferred.or(torrent_link).or(fallback).unwrap_or_default();
        let guid = if id.is_empty() { link.clone() } else { id };
        if link.is_empty()
            || entry_title.len() > MAX_RSS_TEXT_BYTES
            || link.len() > MAX_RSS_TEXT_BYTES
            || guid.len() > MAX_RSS_TEXT_BYTES
        {
            continue;
        }
        items.push(FeedItem {
            title: entry_title,
            link,
            is_torrent,
            guid,
        });
    }
    Ok((title, items))
}

fn is_torrent_content_type(value: &str) -> bool {
    value.trim().to_ascii_lowercase().contains("bittorrent")
}

fn is_torrent_url(value: &str) -> bool {
    let value = value.trim();
    if is_magnet_link(value) {
        return true;
    }
    let without_fragment = value.split('#').next().unwrap_or(value);
    let without_query = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment);
    without_query.to_ascii_lowercase().ends_with(".torrent")
}

pub fn is_magnet_link(value: &str) -> bool {
    value
        .trim()
        .get(..8)
        .map(|prefix| prefix.eq_ignore_ascii_case("magnet:?"))
        .unwrap_or(false)
}

pub fn seen_key(feed_url: &str, guid: &str) -> String {
    let mut digest = crate::sha256::Sha256::new();
    digest.update(&(feed_url.len() as u64).to_be_bytes());
    digest.update(feed_url.as_bytes());
    digest.update(guid.as_bytes());
    let digest = digest.finalize();
    let mut key = String::with_capacity(3 + digest.len() * 2);
    key.push_str("v3:");
    for byte in digest {
        key.push_str(&format!("{byte:02x}"));
    }
    key
}

fn legacy_scoped_seen_key(feed_url: &str, guid: &str) -> String {
    format!("v2:{}:{feed_url}:{guid}", feed_url.len())
}

pub fn remember_seen(seen: &mut Vec<String>, key: String) {
    if seen.iter().any(|existing| existing == &key) {
        return;
    }
    seen.push(key);
    if seen.len() > MAX_SEEN_GUIDS {
        let remove = seen.len() - MAX_SEEN_GUIDS;
        seen.drain(..remove);
    }
}

pub fn match_rules<'a>(
    items: &'a [FeedItem],
    rules: &'a [RssRule],
    seen: &[String],
    feed_url: &str,
) -> Vec<(&'a FeedItem, &'a RssRule)> {
    let mut matches = Vec::new();
    let seen = seen.iter().map(String::as_str).collect::<HashSet<_>>();
    for item in items {
        let scoped_guid = seen_key(feed_url, &item.guid);
        let legacy_scoped_guid = legacy_scoped_seen_key(feed_url, &item.guid);
        if seen.contains(item.guid.as_str())
            || seen.contains(scoped_guid.as_str())
            || seen.contains(legacy_scoped_guid.as_str())
        {
            continue;
        }
        if !item.is_torrent {
            continue;
        }
        for rule in rules {
            if !rule.feed_url.is_empty() && rule.feed_url != feed_url {
                continue;
            }
            if glob_match(&rule.pattern, &item.title) {
                matches.push((item, rule));
                break;
            }
        }
    }
    matches
}

fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern.is_empty() || pattern == "*" {
        return true;
    }
    let text_lower = text.to_ascii_lowercase();
    for alt in pattern.split('|') {
        let alt = alt.trim().to_ascii_lowercase();
        if alt.is_empty() {
            continue;
        }
        if glob_match_single(&alt, &text_lower) {
            return true;
        }
    }
    false
}

fn glob_match_single(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return text.contains(pattern);
    }
    let mut pos = 0;
    for (idx, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if let Some(found) = text[pos..].find(part) {
            if idx == 0 && found != 0 {
                return false;
            }
            pos += found + part.len();
        } else {
            return false;
        }
    }
    if let Some(last) = parts.last() {
        if !last.is_empty() && !text.ends_with(last) {
            return false;
        }
    }
    true
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn save_rss_state(path: &Path, state: &RssState) -> Result<(), String> {
    if state.feeds.len() > MAX_RSS_FEEDS || state.rules.len() > MAX_RSS_RULES {
        return Err("rss save: too many feeds or rules".to_string());
    }
    if state
        .feeds
        .iter()
        .any(|feed| feed.url.len() > MAX_RSS_TEXT_BYTES || feed.title.len() > MAX_RSS_TEXT_BYTES)
        || state.rules.iter().any(|rule| {
            rule.name.len() > MAX_RSS_TEXT_BYTES
                || rule.feed_url.len() > MAX_RSS_TEXT_BYTES
                || rule.pattern.len() > MAX_RSS_PATTERN_BYTES
        })
    {
        return Err("rss save: feed or rule text is too large".to_string());
    }
    let feeds_list: Vec<Value> = state
        .feeds
        .iter()
        .map(|feed| {
            Value::Dict(vec![
                (b"url".to_vec(), Value::Bytes(feed.url.as_bytes().to_vec())),
                (
                    b"title".to_vec(),
                    Value::Bytes(feed.title.as_bytes().to_vec()),
                ),
                (
                    b"last_poll".to_vec(),
                    Value::Int(feed.last_poll.min(i64::MAX as u64) as i64),
                ),
                (
                    b"poll_interval".to_vec(),
                    Value::Int(feed.poll_interval_secs.min(i64::MAX as u64) as i64),
                ),
            ])
        })
        .collect();
    let rules_list: Vec<Value> = state
        .rules
        .iter()
        .map(|rule| {
            Value::Dict(vec![
                (
                    b"name".to_vec(),
                    Value::Bytes(rule.name.as_bytes().to_vec()),
                ),
                (
                    b"feed_url".to_vec(),
                    Value::Bytes(rule.feed_url.as_bytes().to_vec()),
                ),
                (
                    b"pattern".to_vec(),
                    Value::Bytes(rule.pattern.as_bytes().to_vec()),
                ),
            ])
        })
        .collect();
    let seen_list: Vec<Value> = state
        .seen_guids
        .iter()
        .map(|guid| Value::Bytes(guid.as_bytes().to_vec()))
        .collect();
    let dict = Value::Dict(vec![
        (b"feeds".to_vec(), Value::List(feeds_list)),
        (b"rules".to_vec(), Value::List(rules_list)),
        (b"seen".to_vec(), Value::List(seen_list)),
    ]);
    bencode::validate_structure(&dict)
        .map_err(|err| format!("rss save: state structure exceeds parser limits: {err}"))?;
    let data = bencode::encode(&dict);
    write_atomic(path, &data)
}

pub fn load_rss_state(path: &Path) -> Result<RssState, String> {
    let (primary_error, primary_missing) =
        match crate::read_file_limited(path, MAX_RSS_STATE_BYTES, true) {
            Ok(data) => match parse_rss_state(&data) {
                Ok(state) => return Ok(state),
                Err(err) => (err, false),
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                ("rss state file is missing".to_string(), true)
            }
            Err(err) => (format!("rss load: {err}"), false),
        };
    let backup = sidecar_path(path, ".bak");
    let backup_data = match crate::read_file_limited(&backup, MAX_RSS_STATE_BYTES, true) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound && primary_missing => {
            return Err("rss load: state file not found".to_string());
        }
        Err(err) => return Err(format!("rss parse: {primary_error}; backup read: {err}")),
    };
    let state = parse_rss_state(&backup_data)
        .map_err(|backup_error| format!("rss parse: {primary_error}; backup: {backup_error}"))?;
    if let Err(err) = write_atomic_inner(path, &backup_data, false) {
        eprintln!("warning: RSS backup loaded but primary restore failed: {err}");
    }
    Ok(state)
}

pub fn saved_state_exists(path: &Path) -> bool {
    #[cfg(any(unix, windows))]
    if crate::state_dir::is_state_file_path(path) {
        return crate::state_dir::exists(path).unwrap_or(false)
            || crate::state_dir::exists(&sidecar_path(path, ".bak")).unwrap_or(false);
    }
    path.exists() || sidecar_path(path, ".bak").exists()
}

fn parse_rss_state(data: &[u8]) -> Result<RssState, String> {
    let value = bencode::parse(data).map_err(|err| format!("rss parse: {err}"))?;
    let dict = match value {
        Value::Dict(items) => items,
        _ => return Err("rss state not a dict".to_string()),
    };
    let mut state = RssState::new();
    if let Some(Value::List(feeds)) = dict_get(&dict, b"feeds") {
        if feeds.len() > MAX_RSS_FEEDS {
            return Err("rss state has too many feeds".to_string());
        }
        for item in feeds {
            if let Value::Dict(fd) = item {
                let url = dict_get_str(fd, b"url").unwrap_or_default();
                let title = dict_get_str(fd, b"title").unwrap_or_default();
                if url.trim().is_empty()
                    || url.len() > MAX_RSS_TEXT_BYTES
                    || title.len() > MAX_RSS_TEXT_BYTES
                {
                    continue;
                }
                let last_poll = dict_get_int(fd, b"last_poll").unwrap_or(0).max(0) as u64;
                let poll_interval = dict_get_int(fd, b"poll_interval").unwrap_or(900).max(1) as u64;
                state.feeds.push(RssFeed {
                    url,
                    title,
                    items: Vec::new(),
                    last_poll,
                    poll_interval_secs: poll_interval,
                });
            }
        }
    }
    if let Some(Value::List(rules)) = dict_get(&dict, b"rules") {
        if rules.len() > MAX_RSS_RULES {
            return Err("rss state has too many rules".to_string());
        }
        for item in rules {
            if let Value::Dict(rd) = item {
                let name = dict_get_str(rd, b"name").unwrap_or_default();
                let feed_url = dict_get_str(rd, b"feed_url").unwrap_or_default();
                let pattern = dict_get_str(rd, b"pattern").unwrap_or_default();
                if !name.trim().is_empty()
                    && !pattern.trim().is_empty()
                    && name.len() <= MAX_RSS_TEXT_BYTES
                    && feed_url.len() <= MAX_RSS_TEXT_BYTES
                    && pattern.len() <= MAX_RSS_PATTERN_BYTES
                {
                    state.rules.push(RssRule {
                        name,
                        feed_url,
                        pattern,
                    });
                }
            }
        }
    }
    if let Some(Value::List(seen)) = dict_get(&dict, b"seen") {
        let mut unique = HashSet::with_capacity(seen.len().min(MAX_SEEN_GUIDS));
        for item in seen.iter().rev() {
            if state.seen_guids.len() >= MAX_SEEN_GUIDS {
                break;
            }
            if let Value::Bytes(bytes) = item {
                if bytes.is_empty() || bytes.len() > MAX_RSS_TEXT_BYTES {
                    continue;
                }
                if let Ok(s) = String::from_utf8(bytes.clone()) {
                    if unique.insert(s.clone()) {
                        state.seen_guids.push(s);
                    }
                }
            }
        }
        state.seen_guids.reverse();
    }
    Ok(state)
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_owned();
    value.push(suffix);
    PathBuf::from(value)
}

fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    write_atomic_inner(path, data, true)
}

fn write_atomic_inner(path: &Path, data: &[u8], rotate_backup: bool) -> Result<(), String> {
    if data.len() > MAX_RSS_STATE_BYTES {
        return Err("rss save: state file is too large".to_string());
    }
    #[cfg(any(unix, windows))]
    if crate::state_dir::is_state_file_path(path) {
        return crate::state_dir::write_atomic(
            path,
            data,
            rotate_backup,
            0o600,
            MAX_RSS_STATE_BYTES,
        )
        .map_err(|err| format!("rss save state: {err}"));
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if parent != Path::new(".") {
        if crate::state_dir::is_state_file_path(path) {
            let download_dir = parent
                .parent()
                .ok_or_else(|| "rss state directory has no parent".to_string())?;
            crate::ensure_private_state_directory(download_dir)?;
        } else {
            fs::create_dir_all(parent).map_err(|err| format!("rss save dir: {err}"))?;
        }
    }
    let suffix = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let temp = sidecar_path(path, &format!(".tmp-{}-{suffix}", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temp)
            .map_err(|err| format!("rss save temp: {err}"))?;
        file.write_all(data)
            .map_err(|err| format!("rss save: {err}"))?;
        file.sync_all()
            .map_err(|err| format!("rss save sync: {err}"))?;
        drop(file);
        if rotate_backup {
            match fs::symlink_metadata(path) {
                Ok(metadata) => {
                    if metadata.file_type().is_symlink() || !metadata.is_file() {
                        return Err("rss save target is not a regular file".to_string());
                    }
                    rotate_backup_file(path, parent)?;
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(format!("rss save target: {err}")),
            }
        }
        fs::rename(&temp, path).map_err(|err| format!("rss save rename: {err}"))?;
        sync_rss_directory(parent)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn rotate_backup_file(path: &Path, parent: &Path) -> Result<(), String> {
    let backup = sidecar_path(path, ".bak");
    match fs::symlink_metadata(&backup) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err("rss save backup is not a regular file".to_string());
        }
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(format!("rss save backup target: {err}")),
    }

    let mut source_options = OpenOptions::new();
    source_options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        source_options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        source_options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let mut source = source_options
        .open(path)
        .map_err(|err| format!("rss save backup source: {err}"))?;
    let metadata = source
        .metadata()
        .map_err(|err| format!("rss save backup source metadata: {err}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("rss save backup source is not a regular file".to_string());
    }

    let suffix = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let temp = sidecar_path(&backup, &format!(".tmp-{}-{suffix}", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut output = options
            .open(&temp)
            .map_err(|err| format!("rss save backup temp: {err}"))?;
        let copied = std::io::copy(
            &mut Read::by_ref(&mut source).take((MAX_RSS_STATE_BYTES + 1) as u64),
            &mut output,
        )
        .map_err(|err| format!("rss save backup copy: {err}"))?;
        if copied > MAX_RSS_STATE_BYTES as u64 {
            return Err("rss save backup source is too large".to_string());
        }
        output
            .sync_all()
            .map_err(|err| format!("rss save backup sync: {err}"))?;
        drop(output);

        // Renaming a sibling temporary file replaces the directory entry
        // itself; unlike `fs::copy`, it never follows an existing link.
        fs::rename(&temp, &backup).map_err(|err| format!("rss save backup rename: {err}"))?;
        sync_rss_directory(parent)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(unix)]
fn sync_rss_directory(parent: &Path) -> Result<(), String> {
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|err| format!("rss save directory sync: {err}"))
}

#[cfg(not(unix))]
fn sync_rss_directory(_parent: &Path) -> Result<(), String> {
    Ok(())
}

fn dict_get<'a>(dict: &'a [(Vec<u8>, Value)], key: &[u8]) -> Option<&'a Value> {
    dict.iter()
        .find(|(k, _)| k.as_slice() == key)
        .map(|(_, v)| v)
}

fn dict_get_str(dict: &[(Vec<u8>, Value)], key: &[u8]) -> Option<String> {
    match dict_get(dict, key) {
        Some(Value::Bytes(bytes)) => String::from_utf8(bytes.clone()).ok(),
        _ => None,
    }
}

fn dict_get_int(dict: &[(Vec<u8>, Value)], key: &[u8]) -> Option<i64> {
    match dict_get(dict, key) {
        Some(Value::Int(n)) => Some(*n),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rustorrent-rss-{name}-{nanos}.benc"))
    }

    #[test]
    fn parse_rss_feed() {
        let xml = br#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Test Feed</title>
    <item>
      <title>Ubuntu ISO</title>
      <link>http://example.com/ubuntu.torrent</link>
      <guid>guid-001</guid>
    </item>
    <item>
      <title>Debian ISO</title>
      <enclosure url="http://example.com/debian.torrent" type="application/x-bittorrent"/>
      <guid>guid-002</guid>
    </item>
  </channel>
</rss>"#;
        let (title, items) = parse_feed(xml).unwrap();
        assert_eq!(title, "Test Feed");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Ubuntu ISO");
        assert!(items[0].is_torrent);
        assert_eq!(items[1].link, "http://example.com/debian.torrent");
        assert!(items[1].is_torrent);
    }

    #[test]
    fn parse_atom_feed() {
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Atom Feed</title>
  <entry>
    <title>Item One</title>
    <id>urn:uuid:001</id>
    <link href="http://example.com/1.torrent"/>
  </entry>
</feed>"#;
        let (title, items) = parse_feed(xml).unwrap();
        assert_eq!(title, "Atom Feed");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Item One");
        assert!(items[0].is_torrent);
        assert_eq!(items[0].guid, "urn:uuid:001");
    }

    #[test]
    fn atom_prefers_torrent_enclosure_over_alternate_link() {
        let xml = br#"<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Releases</title>
  <entry>
    <title>Release</title>
    <link rel="alternate" href="https://example.com/release"/>
    <link rel="enclosure" type="application/x-bittorrent" href="https://cdn.example.com/download?id=1"/>
  </entry>
</feed>"#;
        let (_, items) = parse_feed(xml).unwrap();
        assert_eq!(items[0].link, "https://cdn.example.com/download?id=1");
        assert!(items[0].is_torrent);
    }

    #[test]
    fn rss_recognizes_magnets_and_torrent_urls_with_queries() {
        let xml = br#"<rss><channel><title>Feed</title>
  <item><title>Magnet</title><link>magnet:?xt=urn:btih:abc</link></item>
  <item><title>File</title><link>https://example.com/file.TORRENT?token=1</link></item>
</channel></rss>"#;
        let (_, items) = parse_feed(xml).unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|item| item.is_torrent));
    }

    #[test]
    fn parses_namespaced_atom_and_rdf_feeds() {
        let atom = br#"<atom:feed xmlns:atom="urn:atom"><atom:title>Atom</atom:title><atom:entry><atom:title>One</atom:title><atom:link href="magnet:?xt=urn:btih:abc"/></atom:entry></atom:feed>"#;
        let (title, items) = parse_feed(atom).unwrap();
        assert_eq!(title, "Atom");
        assert_eq!(items.len(), 1);

        let rdf = br#"<rdf:RDF xmlns:rdf="urn:rdf"><channel><title>RDF</title></channel><item><title>One</title><link>https://example.com/one.torrent</link></item></rdf:RDF>"#;
        let (title, items) = parse_feed(rdf).unwrap();
        assert_eq!(title, "RDF");
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn glob_match_patterns() {
        assert!(glob_match("*ubuntu*", "Ubuntu 24.04 LTS"));
        assert!(glob_match("debian*", "debian-12.iso"));
        assert!(!glob_match("debian*", "Ubuntu 24.04"));
        assert!(glob_match("ubuntu|debian", "My Debian ISO"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("", "anything"));
    }

    #[test]
    fn match_rules_filters_seen_and_non_torrent() {
        let items = vec![
            FeedItem {
                title: "Ubuntu ISO".to_string(),
                link: "http://example.com/ubuntu.torrent".to_string(),
                is_torrent: true,
                guid: "guid-1".to_string(),
            },
            FeedItem {
                title: "Debian ISO".to_string(),
                link: "http://example.com/debian.torrent".to_string(),
                is_torrent: true,
                guid: "guid-2".to_string(),
            },
            FeedItem {
                title: "News article".to_string(),
                link: "http://example.com/news".to_string(),
                is_torrent: false,
                guid: "guid-3".to_string(),
            },
        ];
        let rules = vec![RssRule {
            name: "linux".to_string(),
            feed_url: String::new(),
            pattern: "*".to_string(),
        }];
        let seen = vec!["guid-1".to_string()];
        let matches = match_rules(&items, &rules, &seen, "http://feed");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0.title, "Debian ISO");
    }

    #[test]
    fn seen_guids_are_scoped_to_the_feed() {
        let item = FeedItem {
            title: "Release".to_string(),
            link: "magnet:?xt=urn:btih:abc".to_string(),
            is_torrent: true,
            guid: "shared-guid".to_string(),
        };
        let rule = RssRule {
            name: "all".to_string(),
            feed_url: String::new(),
            pattern: "*".to_string(),
        };
        let seen = vec![seen_key("https://feed-a.example/rss", &item.guid)];
        assert!(match_rules(
            std::slice::from_ref(&item),
            std::slice::from_ref(&rule),
            &seen,
            "https://feed-a.example/rss"
        )
        .is_empty());
        assert_eq!(
            match_rules(
                std::slice::from_ref(&item),
                std::slice::from_ref(&rule),
                &seen,
                "https://feed-b.example/rss"
            )
            .len(),
            1
        );
    }

    #[test]
    fn failed_downloads_remain_eligible_until_success_is_recorded() {
        let item = FeedItem {
            title: "Release".to_string(),
            link: "https://example.com/release.torrent".to_string(),
            is_torrent: true,
            guid: "release-1".to_string(),
        };
        let rule = RssRule {
            name: "all".to_string(),
            feed_url: String::new(),
            pattern: "*".to_string(),
        };
        let feed_url = "https://example.com/feed";
        let mut seen = Vec::new();

        assert_eq!(
            match_rules(
                std::slice::from_ref(&item),
                std::slice::from_ref(&rule),
                &seen,
                feed_url,
            )
            .len(),
            1
        );
        // A failed enqueue/download does not call `remember_seen`, so the next poll retries it.
        assert_eq!(
            match_rules(
                std::slice::from_ref(&item),
                std::slice::from_ref(&rule),
                &seen,
                feed_url,
            )
            .len(),
            1
        );
        remember_seen(&mut seen, seen_key(feed_url, &item.guid));
        assert!(match_rules(
            std::slice::from_ref(&item),
            std::slice::from_ref(&rule),
            &seen,
            feed_url,
        )
        .is_empty());
    }

    #[test]
    fn rss_state_save_load_roundtrip() {
        let path = temp_file("state");
        let mut state = RssState::new();
        state.feeds.push(RssFeed {
            url: "http://example.com/rss".to_string(),
            title: "Test".to_string(),
            items: Vec::new(),
            last_poll: 12345,
            poll_interval_secs: 900,
        });
        state.rules.push(RssRule {
            name: "linux".to_string(),
            feed_url: String::new(),
            pattern: "*ubuntu*".to_string(),
        });
        state.seen_guids.push("guid-001".to_string());

        save_rss_state(&path, &state).unwrap();
        let loaded = load_rss_state(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(loaded.feeds.len(), 1);
        assert_eq!(loaded.feeds[0].url, "http://example.com/rss");
        assert_eq!(loaded.feeds[0].last_poll, 12345);
        assert_eq!(loaded.rules.len(), 1);
        assert_eq!(loaded.rules[0].pattern, "*ubuntu*");
        assert_eq!(loaded.seen_guids, vec!["guid-001"]);
    }

    #[test]
    fn rss_state_roundtrips_at_configured_collection_limits() {
        let path = temp_file("collection-limits");
        let mut state = RssState::new();
        state.feeds = (0..MAX_RSS_FEEDS)
            .map(|index| RssFeed {
                url: format!("https://example.com/{index}"),
                title: "feed".to_string(),
                items: Vec::new(),
                last_poll: 0,
                poll_interval_secs: 900,
            })
            .collect();
        state.rules = (0..MAX_RSS_RULES)
            .map(|index| RssRule {
                name: format!("rule-{index}"),
                feed_url: String::new(),
                pattern: "*".to_string(),
            })
            .collect();

        save_rss_state(&path, &state).unwrap();
        let loaded = load_rss_state(&path).unwrap();
        assert_eq!(loaded.feeds.len(), MAX_RSS_FEEDS);
        assert_eq!(loaded.rules.len(), MAX_RSS_RULES);

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(sidecar_path(&path, ".bak"));
    }

    #[test]
    fn rss_state_save_creates_parent_directory() {
        let path = temp_file("nested");
        let nested = path
            .parent()
            .unwrap()
            .join("rss-state-nested")
            .join("state.benc");
        let _ = fs::remove_dir_all(nested.parent().unwrap());
        let state = RssState::new();
        save_rss_state(&nested, &state).unwrap();
        assert!(nested.exists());
        let _ = fs::remove_file(&nested);
        let _ = fs::remove_dir_all(nested.parent().unwrap());
    }

    #[test]
    fn rss_state_recovers_from_backup() {
        let path = temp_file("backup");
        let mut state = RssState::new();
        state.feeds.push(RssFeed {
            url: "https://example.com/one".to_string(),
            title: "One".to_string(),
            items: Vec::new(),
            last_poll: 1,
            poll_interval_secs: 900,
        });
        save_rss_state(&path, &state).unwrap();
        state.feeds[0].title = "Two".to_string();
        save_rss_state(&path, &state).unwrap();
        fs::write(&path, b"corrupt").unwrap();

        let recovered = load_rss_state(&path).unwrap();
        assert_eq!(recovered.feeds[0].title, "One");

        fs::remove_file(&path).unwrap();
        let recovered_missing_primary = load_rss_state(&path).unwrap();
        assert_eq!(recovered_missing_primary.feeds[0].title, "One");

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(sidecar_path(&path, ".bak"));
    }

    #[cfg(unix)]
    #[test]
    fn rss_state_backup_rotation_rejects_symlinks_without_following_them() {
        use std::os::unix::fs::symlink;

        let path = temp_file("backup-symlink");
        let backup = sidecar_path(&path, ".bak");
        let outside = sidecar_path(&path, ".outside");
        let mut state = RssState::new();
        state.seen_guids.push("first".to_string());
        save_rss_state(&path, &state).unwrap();
        let original = fs::read(&path).unwrap();

        fs::write(&outside, b"must not be overwritten").unwrap();
        symlink(&outside, &backup).unwrap();
        state.seen_guids.push("second".to_string());

        let error = save_rss_state(&path, &state).unwrap_err();
        assert!(error.contains("backup is not a regular file"));
        assert_eq!(fs::read(&outside).unwrap(), b"must not be overwritten");
        assert_eq!(fs::read(&path).unwrap(), original);

        let _ = fs::remove_file(&backup);
        let _ = fs::remove_file(&outside);
        let _ = fs::remove_file(&path);
    }
}
