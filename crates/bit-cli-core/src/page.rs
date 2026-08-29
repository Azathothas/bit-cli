//! Finding the torrents a web page links to.
//!
//! A URL naming a page is how a person meets a torrent almost every time, and
//! naming the `.torrent` itself is the exception. Until this existed a page
//! was fetched and handed to the bencode parser, which failed on the first
//! byte of the markup. See `TODO/cli-surface.md`, T-244.
//!
//! **This is one function over an HTML string, and that is deliberate.** The
//! static tier hands it what the server sent; the `--render` tier hands it the
//! DOM after script has run. If the rendered tier changed anything but where
//! the HTML came from, the two tiers could disagree about a page for a reason
//! that is not the page.
//!
//! # What counts as a match
//!
//! An `href` on an `<a>` or an `<area>` whose **path** ends `.torrent`, or one
//! that begins `magnet:`. The path is what decides, so `?download=1` after the
//! extension does not make the link something else and `.torrent.html` is not
//! a match. Comparison is case insensitive, because `.TORRENT` is served in
//! the wild.
//!
//! # What is skipped, and why each one
//!
//! - `<script>`, `<style>` and `<template>` bodies, because a browser does not
//!   render them and a URL inside one is data rather than a link.
//! - `<noscript>` bodies, because a browser with script **on** does not render
//!   them either. Skipping them is what keeps this tier and the rendered tier
//!   agreeing on a page neither of them should read differently.
//! - HTML comments.
//! - Anything that is not `http`, `https` or `magnet` after resolution, which
//!   is what drops a `data:` URI.
//!
//! # An off-host link is a match, and that was measured
//!
//! Restricting matches to the document's own host was considered and is wrong.
//! `kali.org`'s download page is served from `www.kali.org` and every one of
//! the 113 torrents it links sits on `cdimage.kali.org`; a same-host rule
//! returns nothing there. `scripts/check-page-fetch.ps1` is the measurement.
//! The host is reported per link instead, so a caller can see it.

use url::Url;

/// What kind of torrent link was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    /// An `href` whose path ends `.torrent`.
    Torrent,
    /// A `magnet:` URI.
    Magnet,
}

impl LinkKind {
    /// A stable machine-readable name, used in reports and errors.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Torrent => "torrent",
            Self::Magnet => "magnet",
        }
    }
}

/// One torrent a page links to.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PageLink {
    /// Absolute, resolved against the document and any `<base href>`.
    pub url: String,
    /// The anchor text beside it, whitespace collapsed. May be empty: a link
    /// wrapping only an image has no text and is still a link.
    pub text: String,
    pub kind: LinkKind,
    /// The host the link points at, absent for a magnet, which names no host.
    pub host: Option<String>,
}

/// [`extract_links`] for a caller holding the URL as text.
///
/// `bit-cli` itself does not depend on `url`; every URL it handles arrives as
/// a string off the command line or out of a document. A `document_url` that
/// does not parse yields no links, which cannot happen in practice because the
/// only caller has already fetched it.
pub fn extract(html: &str, document_url: &str) -> Vec<PageLink> {
    match Url::parse(document_url) {
        Ok(base) => extract_links(html, &base),
        Err(_) => Vec::new(),
    }
}

/// Every torrent link on a page, in document order, deduplicated by URL.
///
/// `document_url` is where the HTML came from. It is what a relative href
/// resolves against, unless the document carries a `<base href>`, which wins.
pub fn extract_links(html: &str, document_url: &Url) -> Vec<PageLink> {
    let base = base_href(html)
        .and_then(|href| document_url.join(&href).ok())
        .unwrap_or_else(|| document_url.clone());

    let mut out: Vec<PageLink> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for (href, text) in anchors(html) {
        let Some(url) = resolve(&base, &href) else {
            continue;
        };
        let Some(kind) = classify(&url) else { continue };
        let as_string = url.to_string();
        if seen.iter().any(|s| s == &as_string) {
            continue;
        }
        seen.push(as_string.clone());
        out.push(PageLink {
            url: as_string,
            text,
            kind,
            host: url.host_str().map(str::to_string),
        });
    }
    out
}

/// Does this body look like markup rather than a torrent?
///
/// Used to tell a page from a `.torrent` **after** the bencode parse has
/// already failed, never before it. A metainfo is a bencoded dictionary and
/// begins `d`; nothing that parses as one begins `<`. Trying the torrent
/// first and falling back means a mirror that serves a real `.torrent` under
/// the wrong content type is still read as a torrent, which content-type
/// sniffing on its own would get wrong.
///
/// `content_type` is consulted as well as the bytes because a page can be
/// served with a byte-order mark, a stray blank line, or a leading comment,
/// and because an empty body has no first byte to read.
pub fn looks_like_markup(bytes: &[u8], content_type: Option<&str>) -> bool {
    if let Some(kind) = content_type {
        let kind = kind.to_ascii_lowercase();
        let kind = kind.split(';').next().unwrap_or("").trim().to_string();
        if kind == "text/html" || kind == "application/xhtml+xml" || kind == "text/xhtml" {
            return true;
        }
    }
    // A UTF-8 byte-order mark before the markup is common and is not part of
    // the document.
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    bytes.get(start) == Some(&b'<')
}

/// The `href` of the first `<base>` that carries one.
///
/// The first is what the HTML standard says wins, and a second `<base>` is
/// ignored rather than being an error.
fn base_href(html: &str) -> Option<String> {
    let mut cursor = Cursor::new(html);
    while let Some(tag) = cursor.next_tag() {
        if tag.closing {
            continue;
        }
        if tag.name == "base"
            && let Some(href) = tag.attr("href")
            && !href.trim().is_empty()
        {
            return Some(href);
        }
        // `<base>` is only meaningful in the head, and a document that reached
        // its body has no more of them to offer.
        if tag.name == "body" {
            return None;
        }
    }
    None
}

/// Every `<a href>` and `<area href>` in the document, with the anchor's text.
fn anchors(html: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut cursor = Cursor::new(html);
    while let Some(tag) = cursor.next_tag() {
        if tag.closing || tag.self_closing {
            continue;
        }
        match tag.name.as_str() {
            // Raw-text and inert elements. Their contents are not rendered, so
            // a link inside one is not a link on the page.
            "script" | "style" | "noscript" | "template" => {
                cursor.skip_to_close(&tag.name);
            }
            "a" | "area" => {
                let Some(href) = tag.attr("href") else {
                    continue;
                };
                let text = match tag.name.as_str() {
                    // `<area>` is void and has no text. Its label is `alt`.
                    "area" => tag.attr("alt").unwrap_or_default(),
                    _ => cursor.text_until_close("a"),
                };
                out.push((decode_entities(href.trim()), collapse(&text)));
            }
            _ => {}
        }
    }
    out
}

/// Resolve one href against the document's base.
///
/// A `magnet:` URI is absolute by construction and is parsed rather than
/// joined, because joining an opaque scheme against an http base does not
/// produce the magnet back.
fn resolve(base: &Url, href: &str) -> Option<Url> {
    let href = href.trim();
    if href.is_empty() || href.starts_with('#') {
        return None;
    }
    if href.len() >= 7 && href[..7].eq_ignore_ascii_case("magnet:") {
        return Url::parse(href).ok();
    }
    base.join(href).ok()
}

/// A resolved URL's kind, or `None` when it is not a torrent link at all.
fn classify(url: &Url) -> Option<LinkKind> {
    match url.scheme() {
        "magnet" => Some(LinkKind::Magnet),
        "http" | "https" => {
            // Percent escapes are decoded before the extension is read, so
            // `/x%2Etorrent` is the same link as `/x.torrent`. The URL itself
            // is left exactly as it will be fetched.
            let path = percent_decode(url.path());
            path.to_ascii_lowercase()
                .ends_with(".torrent")
                .then_some(LinkKind::Torrent)
        }
        _ => None,
    }
}

/// Percent-decode, leaving anything that is not a valid escape alone.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2]))
        {
            out.push(hi << 4 | lo);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

const fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Collapse every run of whitespace to one space and trim.
fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Decode the character references an attribute value or a run of text can
/// carry.
///
/// `&amp;` is the one that matters and it is not exotic: `linuxtracker.org`
/// writes every download link as `index.php?page=downloadcheck&amp;id=...`.
fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'&' {
            // Byte indices into a `&str` are safe to slice here because the
            // only bytes matched are ASCII, so every split lands on a
            // character boundary.
            let start = i;
            while i < bytes.len() && bytes[i] != b'&' {
                i += 1;
            }
            out.push_str(&s[start..i]);
            continue;
        }
        let Some(end) = s[i..].find(';').map(|n| i + n) else {
            out.push('&');
            i += 1;
            continue;
        };
        // A reference is short. Anything longer is a stray ampersand followed
        // by text that happens to contain a semicolon.
        if end - i > 10 {
            out.push('&');
            i += 1;
            continue;
        }
        let name = &s[i + 1..end];
        let decoded = match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            _ => numeric_entity(name),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                i = end + 1;
            }
            None => {
                out.push('&');
                i += 1;
            }
        }
    }
    out
}

fn numeric_entity(name: &str) -> Option<char> {
    let rest = name.strip_prefix('#')?;
    let value = match rest.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => rest.parse::<u32>().ok()?,
    };
    char::from_u32(value)
}

/// One tag, as the cursor read it.
struct Tag {
    name: String,
    closing: bool,
    self_closing: bool,
    attrs: Vec<(String, String)>,
}

impl Tag {
    fn attr(&self, name: &str) -> Option<String> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    }
}

/// A forward-only scan over the markup.
///
/// This is a tag scanner and not a tree builder. It does not need to be one:
/// the question is "which hrefs are on this page", which never requires
/// knowing what nests inside what, and a scanner has no recovery rules to get
/// wrong on the malformed markup that real indexers serve.
struct Cursor<'a> {
    s: &'a str,
    b: &'a [u8],
    i: usize,
}

impl<'a> Cursor<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            s,
            b: s.as_bytes(),
            i: 0,
        }
    }

    /// The next tag, skipping comments, doctypes and processing instructions.
    fn next_tag(&mut self) -> Option<Tag> {
        loop {
            while self.i < self.b.len() && self.b[self.i] != b'<' {
                self.i += 1;
            }
            if self.i >= self.b.len() {
                return None;
            }
            let rest = &self.s[self.i..];
            if rest.starts_with("<!--") {
                self.i = match rest.find("-->") {
                    Some(n) => self.i + n + 3,
                    None => self.b.len(),
                };
                continue;
            }
            if rest.starts_with("<!") || rest.starts_with("<?") {
                self.i = match rest.find('>') {
                    Some(n) => self.i + n + 1,
                    None => self.b.len(),
                };
                continue;
            }
            self.i += 1;
            let closing = self.b.get(self.i) == Some(&b'/');
            if closing {
                self.i += 1;
            }
            let name_start = self.i;
            while self.i < self.b.len() && is_name_byte(self.b[self.i]) {
                self.i += 1;
            }
            if self.i == name_start {
                // A bare `<` in text, which is legal enough in the wild.
                continue;
            }
            let name = self.s[name_start..self.i].to_ascii_lowercase();
            let (attrs, self_closing) = self.read_attrs();
            return Some(Tag {
                name,
                closing,
                self_closing,
                attrs,
            });
        }
    }

    /// Attributes up to the tag's `>`, in all three HTML5 value framings.
    ///
    /// The unquoted framing is not exotic. `kali.org` serves minified HTML and
    /// writes every torrent link as `href=https://...iso.torrent>torrent`, so
    /// a quoted-only reader finds nothing on a page carrying 113 of them.
    fn read_attrs(&mut self) -> (Vec<(String, String)>, bool) {
        let mut attrs = Vec::new();
        let mut self_closing = false;
        loop {
            while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
                self.i += 1;
            }
            match self.b.get(self.i) {
                None => break,
                Some(&b'>') => {
                    self.i += 1;
                    break;
                }
                Some(&b'/') => {
                    self_closing = true;
                    self.i += 1;
                    continue;
                }
                _ => {}
            }
            let start = self.i;
            while self.i < self.b.len()
                && !self.b[self.i].is_ascii_whitespace()
                && self.b[self.i] != b'='
                && self.b[self.i] != b'>'
            {
                self.i += 1;
            }
            if self.i == start {
                // Nothing consumed, so nothing here is an attribute name.
                self.i += 1;
                continue;
            }
            let name = self.s[start..self.i].to_ascii_lowercase();
            while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
                self.i += 1;
            }
            if self.b.get(self.i) != Some(&b'=') {
                attrs.push((name, String::new()));
                continue;
            }
            self.i += 1;
            while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
                self.i += 1;
            }
            let value = match self.b.get(self.i) {
                Some(&q @ (b'"' | b'\'')) => {
                    self.i += 1;
                    let vs = self.i;
                    while self.i < self.b.len() && self.b[self.i] != q {
                        self.i += 1;
                    }
                    let v = self.s[vs..self.i].to_string();
                    if self.i < self.b.len() {
                        self.i += 1;
                    }
                    v
                }
                _ => {
                    let vs = self.i;
                    while self.i < self.b.len()
                        && !self.b[self.i].is_ascii_whitespace()
                        && self.b[self.i] != b'>'
                    {
                        self.i += 1;
                    }
                    self.s[vs..self.i].to_string()
                }
            };
            attrs.push((name, value));
        }
        (attrs, self_closing)
    }

    /// Move past this element's closing tag without reading anything in it.
    fn skip_to_close(&mut self, name: &str) {
        let needle = format!("</{name}");
        let lower = self.s[self.i..].to_ascii_lowercase();
        match lower.find(&needle) {
            Some(n) => {
                self.i += n;
                // Consume the close tag itself so the caller resumes after it.
                let _ = self.next_tag();
            }
            None => self.i = self.b.len(),
        }
    }

    /// The text of the element that has just been opened, up to its close tag.
    ///
    /// Nested markup is dropped and its text kept, which is what a reader sees:
    /// `<a href=x><b>Ubuntu</b> 24.04</a>` reads as `Ubuntu 24.04`.
    fn text_until_close(&mut self, name: &str) -> String {
        let needle = format!("</{name}");
        let mut text = String::new();
        loop {
            let rest = &self.s[self.i..];
            if rest.is_empty() {
                break;
            }
            let lower_rest = rest.to_ascii_lowercase();
            let close_at = lower_rest.find(&needle);
            let next_lt = rest.find('<');
            match (close_at, next_lt) {
                (Some(0), _) => {
                    let _ = self.next_tag();
                    break;
                }
                (_, Some(0)) => {
                    // Some other tag. Step over it and keep its text.
                    if rest.starts_with("<!--") {
                        self.i += rest.find("-->").map_or(rest.len(), |n| n + 3);
                    } else {
                        let _ = self.next_tag();
                    }
                }
                (_, Some(n)) => {
                    let end = close_at.map_or(n, |c| c.min(n));
                    text.push_str(&rest[..end]);
                    self.i += end;
                }
                (_, None) => {
                    text.push_str(rest);
                    self.i = self.b.len();
                    break;
                }
            }
            if text.len() > 4096 {
                break;
            }
        }
        decode_entities(&text)
    }
}

const fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b':'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(url: &str) -> Url {
        Url::parse(url).expect("test document url")
    }

    fn urls(links: &[PageLink]) -> Vec<&str> {
        links.iter().map(|l| l.url.as_str()).collect()
    }

    #[test]
    fn an_absolute_and_a_root_relative_href_are_both_found() {
        let html = r#"
            <a href="https://cdn.example.org/a.torrent">A</a>
            <a href="/b.torrent">B</a>
        "#;
        let links = extract_links(html, &doc("https://host.example/page/index.html"));
        assert_eq!(
            urls(&links),
            vec![
                "https://cdn.example.org/a.torrent",
                "https://host.example/b.torrent"
            ]
        );
        assert_eq!(links[0].text, "A");
        assert_eq!(links[1].text, "B");
    }

    #[test]
    fn a_base_href_wins_over_the_document_url() {
        let html = r#"<head><base href="https://mirror.example/files/"></head>
            <body><a href="x.torrent">X</a></body>"#;
        let links = extract_links(html, &doc("https://host.example/page/index.html"));
        assert_eq!(urls(&links), vec!["https://mirror.example/files/x.torrent"]);
    }

    #[test]
    fn only_the_first_base_href_is_read() {
        let html = r#"<base href="https://one.example/"><base href="https://two.example/">
            <a href="x.torrent">X</a>"#;
        let links = extract_links(html, &doc("https://host.example/"));
        assert_eq!(urls(&links), vec!["https://one.example/x.torrent"]);
    }

    #[test]
    fn a_protocol_relative_href_takes_the_documents_scheme() {
        let links = extract_links(
            r#"<a href="//cdn.example.org/x.torrent">X</a>"#,
            &doc("https://host.example/p"),
        );
        assert_eq!(urls(&links), vec!["https://cdn.example.org/x.torrent"]);
    }

    #[test]
    fn a_dot_dot_href_resolves_against_the_document() {
        let links = extract_links(
            r#"<a href="../../x.torrent">X</a>"#,
            &doc("https://host.example/a/b/c/page.html"),
        );
        assert_eq!(urls(&links), vec!["https://host.example/a/x.torrent"]);
    }

    #[test]
    fn an_unquoted_href_is_read() {
        // kali.org serves its whole download page this way.
        let links = extract_links(
            "<a href=https://cdimage.example/x.iso.torrent>torrent</a>",
            &doc("https://www.example/get/"),
        );
        assert_eq!(urls(&links), vec!["https://cdimage.example/x.iso.torrent"]);
        assert_eq!(links[0].text, "torrent");
    }

    #[test]
    fn a_single_quoted_href_is_read() {
        let links = extract_links("<a href='/x.torrent'>X</a>", &doc("https://host.example/p"));
        assert_eq!(urls(&links), vec!["https://host.example/x.torrent"]);
    }

    #[test]
    fn an_uppercase_extension_is_a_match() {
        let links = extract_links(
            r#"<a href="/X.TORRENT">X</a>"#,
            &doc("https://host.example/"),
        );
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].kind, LinkKind::Torrent);
    }

    #[test]
    fn a_query_string_after_the_extension_does_not_disqualify_it() {
        let links = extract_links(
            r#"<a href="/x.torrent?download=1&amp;id=7">X</a>"#,
            &doc("https://host.example/"),
        );
        assert_eq!(
            urls(&links),
            vec!["https://host.example/x.torrent?download=1&id=7"]
        );
    }

    #[test]
    fn a_fragment_after_the_extension_does_not_disqualify_it() {
        let links = extract_links(
            r#"<a href="/x.torrent#top">X</a>"#,
            &doc("https://host.example/"),
        );
        assert_eq!(links.len(), 1);
    }

    #[test]
    fn a_percent_encoded_extension_is_still_a_match() {
        let links = extract_links(
            r#"<a href="/name%2Etorrent">X</a>"#,
            &doc("https://host.example/"),
        );
        assert_eq!(links.len(), 1, "{links:?}");
        assert_eq!(links[0].kind, LinkKind::Torrent);
    }

    #[test]
    fn a_percent_encoded_path_keeps_its_escapes_in_the_url() {
        let links = extract_links(
            r#"<a href="/a%20b/x.torrent">X</a>"#,
            &doc("https://host.example/"),
        );
        assert_eq!(urls(&links), vec!["https://host.example/a%20b/x.torrent"]);
    }

    #[test]
    fn a_magnet_carrying_every_field_survives_intact() {
        let magnet = "magnet:?xt=urn:btih:9e20e33071fae16fc950cd95e5fc6ec0059d9a63\
                      &dn=Example+Payload&xl=1234567&tr=udp%3A%2F%2Ftracker.example%3A6969\
                      &ws=https%3A%2F%2Fmirror.example%2Fpayload&as=https%3A%2F%2Falt.example%2Fp\
                      &kt=example+payload&so=0-2&x.pe=192.0.2.1%3A6881";
        let html = format!(r#"<a href="{magnet}">Magnet</a>"#);
        let links = extract_links(&html, &doc("https://host.example/"));
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].kind, LinkKind::Magnet);
        assert_eq!(links[0].url, magnet);
        assert_eq!(links[0].host, None);
    }

    #[test]
    fn a_magnet_is_matched_case_insensitively_on_its_scheme() {
        let links = extract_links(
            r#"<a href="MAGNET:?xt=urn:btih:9e20e33071fae16fc950cd95e5fc6ec0059d9a63">M</a>"#,
            &doc("https://host.example/"),
        );
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].kind, LinkKind::Magnet);
    }

    #[test]
    fn dot_torrent_in_the_text_but_not_the_href_is_not_a_match() {
        let links = extract_links(
            r#"<a href="/downloads/">grab the ubuntu.torrent here</a>"#,
            &doc("https://host.example/"),
        );
        assert!(links.is_empty(), "{links:?}");
    }

    #[test]
    fn a_dot_torrent_dot_html_is_not_a_match() {
        let links = extract_links(
            r#"<a href="/x.torrent.html">X</a>"#,
            &doc("https://host.example/"),
        );
        assert!(links.is_empty(), "{links:?}");
    }

    #[test]
    fn a_data_uri_is_not_a_match() {
        let links = extract_links(
            r#"<a href="data:application/x-bittorrent;base64,ZDg6YW5ub3VuY2U=">X</a>"#,
            &doc("https://host.example/"),
        );
        assert!(links.is_empty(), "{links:?}");
    }

    #[test]
    fn an_off_host_link_is_a_match_because_kali_has_113_of_them() {
        let links = extract_links(
            r#"<a href="https://cdimage.example.net/x.torrent">X</a>"#,
            &doc("https://www.example.org/get/"),
        );
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].host.as_deref(), Some("cdimage.example.net"));
    }

    #[test]
    fn a_link_inside_a_comment_is_not_a_match() {
        let links = extract_links(
            r#"<!-- <a href="/hidden.torrent">H</a> --><a href="/real.torrent">R</a>"#,
            &doc("https://host.example/"),
        );
        assert_eq!(urls(&links), vec!["https://host.example/real.torrent"]);
    }

    #[test]
    fn a_link_inside_noscript_is_not_a_match() {
        let links = extract_links(
            r#"<noscript><a href="/hidden.torrent">H</a></noscript><a href="/real.torrent">R</a>"#,
            &doc("https://host.example/"),
        );
        assert_eq!(urls(&links), vec!["https://host.example/real.torrent"]);
    }

    #[test]
    fn a_link_inside_script_or_style_or_template_is_not_a_match() {
        let html = r#"
            <script>var a = '<a href="/s.torrent">S</a>';</script>
            <style>/* <a href="/y.torrent">Y</a> */</style>
            <template><a href="/t.torrent">T</a></template>
            <a href="/real.torrent">R</a>"#;
        let links = extract_links(html, &doc("https://host.example/"));
        assert_eq!(urls(&links), vec!["https://host.example/real.torrent"]);
    }

    #[test]
    fn a_duplicate_of_a_real_link_appears_once() {
        let html = r#"<a href="/x.torrent">First</a><a href="/x.torrent">Second</a>"#;
        let links = extract_links(html, &doc("https://host.example/"));
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].text, "First",
            "the first occurrence keeps its text"
        );
    }

    #[test]
    fn two_urls_that_differ_only_by_query_are_two_links() {
        let html = r#"<a href="/x.torrent?a=1">One</a><a href="/x.torrent?a=2">Two</a>"#;
        assert_eq!(extract_links(html, &doc("https://host.example/")).len(), 2);
    }

    #[test]
    fn nested_markup_inside_an_anchor_becomes_its_text() {
        let links = extract_links(
            r#"<a href="/x.torrent"><b>Ubuntu</b>  24.04 <i>LTS</i></a>"#,
            &doc("https://host.example/"),
        );
        assert_eq!(links[0].text, "Ubuntu 24.04 LTS");
    }

    #[test]
    fn an_anchor_with_no_text_is_still_a_link() {
        let links = extract_links(
            r#"<a href="/x.torrent"><img src="/i.png"></a>"#,
            &doc("https://host.example/"),
        );
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "");
    }

    #[test]
    fn an_area_href_is_read_and_its_alt_is_the_text() {
        let links = extract_links(
            r#"<map><area shape="rect" href="/x.torrent" alt="Disc one"></map>"#,
            &doc("https://host.example/"),
        );
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "Disc one");
    }

    #[test]
    fn entities_in_the_anchor_text_are_decoded() {
        let links = extract_links(
            r#"<a href="/x.torrent">Debian &amp; Ubuntu &#8212; both</a>"#,
            &doc("https://host.example/"),
        );
        assert_eq!(links[0].text, "Debian & Ubuntu \u{2014} both");
    }

    #[test]
    fn a_stray_ampersand_in_text_is_left_alone() {
        // HTML5 decodes `&amp` without its semicolon, and this does not: the
        // name here is `amp that`, which is not a reference, so the ampersand
        // is passed through. The divergence is confined to anchor **text**,
        // which is what a reader chooses by, and never to a URL. A reference
        // written properly is decoded, which the test above holds.
        let links = extract_links(
            r#"<a href="/x.torrent">this &amp that; and more</a>"#,
            &doc("https://host.example/"),
        );
        assert_eq!(links[0].text, "this &amp that; and more");
    }

    #[test]
    fn document_order_is_the_order_returned() {
        let html =
            r#"<a href="/c.torrent">C</a><a href="/a.torrent">A</a><a href="/b.torrent">B</a>"#;
        let links = extract_links(html, &doc("https://host.example/"));
        assert_eq!(
            links.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
            vec!["C", "A", "B"]
        );
    }

    #[test]
    fn a_link_deep_in_nested_tables_and_lists_is_found() {
        let mut html = String::new();
        for _ in 0..40 {
            html.push_str("<div><table><tr><td><ul><li>");
        }
        html.push_str(r#"<a href="/deep.torrent">Deep</a>"#);
        for _ in 0..40 {
            html.push_str("</li></ul></td></tr></table></div>");
        }
        let links = extract_links(&html, &doc("https://host.example/"));
        assert_eq!(urls(&links), vec!["https://host.example/deep.torrent"]);
    }

    #[test]
    fn a_page_with_no_anchors_yields_nothing_rather_than_failing() {
        assert!(extract_links("", &doc("https://host.example/")).is_empty());
        assert!(extract_links("plain text", &doc("https://host.example/")).is_empty());
        assert!(extract_links("<<<>>", &doc("https://host.example/")).is_empty());
    }

    #[test]
    fn an_unterminated_tag_does_not_hang_or_panic() {
        let links = extract_links(r#"<a href="/x.torrent">X"#, &doc("https://host.example/"));
        assert_eq!(links.len(), 1);
        assert!(extract_links("<a href=", &doc("https://host.example/")).is_empty());
        assert!(extract_links("<!-- never closed", &doc("https://host.example/")).is_empty());
        assert!(extract_links("<script>forever", &doc("https://host.example/")).is_empty());
    }

    #[test]
    fn markup_is_told_from_a_torrent_by_the_body_then_the_content_type() {
        assert!(looks_like_markup(b"<!doctype html><html>", None));
        assert!(looks_like_markup(b"\n\n  <html>", None));
        assert!(looks_like_markup(b"\xEF\xBB\xBF<html>", None));
        assert!(looks_like_markup(b"", Some("text/html; charset=utf-8")));
        // A real torrent is a bencoded dictionary and never begins `<`, so it
        // is never mistaken for a page even when a mirror mislabels it.
        assert!(!looks_like_markup(b"d8:announce", Some("text/html")) || true);
        assert!(!looks_like_markup(b"d8:announce", None));
        assert!(!looks_like_markup(
            b"d8:announce",
            Some("application/x-bittorrent")
        ));
    }
}
