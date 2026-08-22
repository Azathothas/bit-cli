use std::collections::HashSet;

pub struct XmlNode {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub text: String,
    pub children: Vec<XmlNode>,
}

impl XmlNode {
    pub fn child(&self, tag: &str) -> Option<&XmlNode> {
        self.children
            .iter()
            .find(|child| child.tag == tag || child.local_name() == tag)
    }

    pub fn children_by_tag(&self, tag: &str) -> Vec<&XmlNode> {
        self.children
            .iter()
            .filter(|child| child.tag == tag || child.local_name() == tag)
            .collect()
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn local_name(&self) -> &str {
        self.tag
            .rsplit_once(':')
            .map(|(_, name)| name)
            .unwrap_or(&self.tag)
    }
}

const MAX_XML_BYTES: usize = 8 * 1024 * 1024;
const MAX_XML_DEPTH: usize = 128;
const MAX_XML_NODES: usize = 100_000;
const MAX_XML_STORED_TEXT_BYTES: usize = 16 * 1024 * 1024;
const MAX_XML_ATTRIBUTES_PER_ELEMENT: usize = 256;
const MAX_XML_ATTRIBUTES: usize = 16_384;
const MAX_XML_STORED_ATTRIBUTE_BYTES: usize = 4 * 1024 * 1024;

pub fn parse(data: &[u8]) -> Option<XmlNode> {
    if data.len() > MAX_XML_BYTES {
        return None;
    }
    let text = std::str::from_utf8(data).ok()?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut parser = Parser {
        text,
        pos: 0,
        nodes: 0,
        stored_text_bytes: 0,
        attributes: 0,
        stored_attribute_bytes: 0,
    };
    parser.skip_misc(true)?;
    let root = parser.parse_element(0)?;
    parser.skip_misc(false)?;
    (parser.pos == parser.text.len()).then_some(root)
}

struct Parser<'a> {
    text: &'a str,
    pos: usize,
    nodes: usize,
    stored_text_bytes: usize,
    attributes: usize,
    stored_attribute_bytes: usize,
}

impl Parser<'_> {
    fn remaining(&self) -> &str {
        &self.text[self.pos..]
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.remaining().chars().next() {
            if !ch.is_whitespace() {
                break;
            }
            self.pos += ch.len_utf8();
        }
    }

    fn skip_misc(&mut self, allow_doctype: bool) -> Option<()> {
        loop {
            self.skip_whitespace();
            if self.remaining().starts_with("<!--") {
                self.skip_comment()?;
            } else if self.remaining().starts_with("<?") {
                self.skip_processing_instruction()?;
            } else if allow_doctype
                && (self.remaining().starts_with("<!DOCTYPE")
                    || self.remaining().starts_with("<!doctype"))
            {
                self.skip_doctype()?;
            } else {
                return Some(());
            }
        }
    }

    fn skip_comment(&mut self) -> Option<()> {
        let end = self.remaining().find("-->")?;
        self.pos += end + 3;
        Some(())
    }

    fn skip_processing_instruction(&mut self) -> Option<()> {
        let end = self.remaining().find("?>")?;
        self.pos += end + 2;
        Some(())
    }

    fn skip_doctype(&mut self) -> Option<()> {
        let mut quote = None;
        let mut subset_depth = 0usize;
        for (offset, ch) in self.remaining().char_indices() {
            if let Some(active_quote) = quote {
                if ch == active_quote {
                    quote = None;
                }
                continue;
            }
            match ch {
                '\'' | '"' => quote = Some(ch),
                '[' => subset_depth = subset_depth.saturating_add(1),
                ']' => subset_depth = subset_depth.saturating_sub(1),
                '>' if subset_depth == 0 => {
                    self.pos += offset + 1;
                    return Some(());
                }
                _ => {}
            }
        }
        None
    }

    fn parse_name(&mut self) -> Option<String> {
        let start = self.pos;
        while let Some(ch) = self.remaining().chars().next() {
            if ch.is_whitespace() || matches!(ch, '>' | '/' | '=' | '<' | '?' | '!') {
                break;
            }
            self.pos += ch.len_utf8();
        }
        let name = self.text.get(start..self.pos)?;
        let mut chars = name.chars();
        let first = chars.next()?;
        if !is_name_start(first) || chars.any(|ch| !is_name_continue(ch)) {
            return None;
        }
        Some(name.to_string())
    }

    fn append_text(&mut self, output: &mut String, value: &str) -> Option<()> {
        self.stored_text_bytes = self.stored_text_bytes.checked_add(value.len())?;
        if self.stored_text_bytes > MAX_XML_STORED_TEXT_BYTES {
            return None;
        }
        output.push_str(value);
        Some(())
    }

    fn parse_element(&mut self, depth: usize) -> Option<XmlNode> {
        if depth >= MAX_XML_DEPTH || self.nodes >= MAX_XML_NODES {
            return None;
        }
        self.skip_whitespace();
        while self.remaining().starts_with("<!--") {
            self.skip_comment()?;
            self.skip_whitespace();
        }
        if !self.remaining().starts_with('<')
            || self.remaining().starts_with("</")
            || self.remaining().starts_with("<!")
            || self.remaining().starts_with("<?")
        {
            return None;
        }
        self.pos += 1;
        let tag = self.parse_name()?;
        self.nodes += 1;

        let mut attrs = Vec::new();
        let mut attribute_names = HashSet::new();
        loop {
            self.skip_whitespace();
            if self.remaining().starts_with("/>") {
                self.pos += 2;
                return Some(XmlNode {
                    tag,
                    attrs,
                    text: String::new(),
                    children: Vec::new(),
                });
            }
            if self.remaining().starts_with('>') {
                self.pos += 1;
                break;
            }
            let key = self.parse_name()?;
            if attrs.len() >= MAX_XML_ATTRIBUTES_PER_ELEMENT
                || self.attributes >= MAX_XML_ATTRIBUTES
                || !attribute_names.insert(key.clone())
            {
                return None;
            }
            self.skip_whitespace();
            if !self.remaining().starts_with('=') {
                return None;
            }
            self.pos += 1;
            self.skip_whitespace();
            let quote = self.remaining().chars().next()?;
            if quote != '"' && quote != '\'' {
                return None;
            }
            self.pos += quote.len_utf8();
            let value_start = self.pos;
            let value_end = self.remaining().find(quote)?;
            let raw_value = &self.text[value_start..value_start + value_end];
            if raw_value.contains('<') {
                return None;
            }
            let source_bytes = key.len().checked_add(raw_value.len())?;
            if self.stored_attribute_bytes.checked_add(source_bytes)?
                > MAX_XML_STORED_ATTRIBUTE_BYTES
            {
                return None;
            }
            let value = decode_entities(raw_value);
            let stored_bytes = key.len().checked_add(value.len())?;
            self.stored_attribute_bytes = self.stored_attribute_bytes.checked_add(stored_bytes)?;
            self.attributes += 1;
            self.pos += value_end + quote.len_utf8();
            attrs.push((key, value));
        }

        let mut children = Vec::new();
        let mut text_buf = String::new();

        loop {
            if self.pos >= self.text.len() {
                return None;
            }

            if self.remaining().starts_with("<![CDATA[") {
                self.pos += 9;
                let end = self.remaining().find("]]>")?;
                let cdata = self.remaining()[..end].to_string();
                self.append_text(&mut text_buf, &cdata)?;
                self.pos += end + 3;
                continue;
            }

            if self.remaining().starts_with("<!--") {
                self.skip_comment()?;
                continue;
            }

            if self.remaining().starts_with("<?") {
                self.skip_processing_instruction()?;
                continue;
            }

            if self.remaining().starts_with("</") {
                self.pos += 2;
                let close_tag = self.parse_name()?;
                self.skip_whitespace();
                if close_tag != tag || !self.remaining().starts_with('>') {
                    return None;
                }
                self.pos += 1;
                return Some(XmlNode {
                    tag,
                    attrs,
                    text: text_buf,
                    children,
                });
            }

            if self.remaining().starts_with('<') {
                let child = self.parse_element(depth + 1)?;
                // Most consumers want the human-readable value of mixed-content nodes. Keep
                // child nodes for structured access while also preserving their text in order.
                self.append_text(&mut text_buf, &child.text)?;
                children.push(child);
                continue;
            }

            let end = self.remaining().find('<').unwrap_or(self.remaining().len());
            if end == 0 {
                return None;
            }
            let decoded = decode_entities(&self.remaining()[..end]);
            self.append_text(&mut text_buf, &decoded)?;
            self.pos += end;
        }
    }
}

fn is_name_start(ch: char) -> bool {
    ch == ':' || ch == '_' || ch.is_alphabetic()
}

fn is_name_continue(ch: char) -> bool {
    is_name_start(ch) || ch.is_alphanumeric() || matches!(ch, '-' | '.')
}

fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut pos = 0usize;
    while let Some(relative) = input[pos..].find('&') {
        let amp = pos + relative;
        out.push_str(&input[pos..amp]);
        let after_amp = amp + 1;
        // Entity names are capped below, so never scan the untrusted suffix
        // looking for a distant semicolon. Repeating '&' before one final ';'
        // would otherwise rescan nearly the entire suffix for every byte.
        let Some(end_relative) = input.as_bytes()[after_amp..]
            .iter()
            .take(33)
            .position(|byte| *byte == b';')
        else {
            out.push('&');
            pos = after_amp;
            continue;
        };
        if end_relative == 0 || end_relative > 32 {
            out.push('&');
            pos = after_amp;
            continue;
        }
        let entity = &input[after_amp..after_amp + end_relative];
        match entity {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            _ if entity.starts_with('#') => {
                let code = if entity.starts_with("#x") || entity.starts_with("#X") {
                    u32::from_str_radix(&entity[2..], 16).ok()
                } else {
                    entity[1..].parse::<u32>().ok()
                };
                if let Some(c) = code.and_then(char::from_u32) {
                    out.push(c);
                } else {
                    out.push('&');
                    out.push_str(entity);
                    out.push(';');
                }
            }
            _ => {
                out.push('&');
                out.push_str(entity);
                out.push(';');
            }
        }
        pos = after_amp + end_relative + 1;
    }
    out.push_str(&input[pos..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_element() {
        let xml = b"<root><child>hello</child></root>";
        let node = parse(xml).unwrap();
        assert_eq!(node.tag, "root");
        assert_eq!(node.children.len(), 1);
        assert_eq!(node.children[0].tag, "child");
        assert_eq!(node.children[0].text, "hello");
    }

    #[test]
    fn parse_attributes() {
        let xml = b"<item url=\"https://example.com\" type='text/html'/>";
        let node = parse(xml).unwrap();
        assert_eq!(node.tag, "item");
        assert_eq!(node.attr("url"), Some("https://example.com"));
        assert_eq!(node.attr("type"), Some("text/html"));
        assert!(node.children.is_empty());
    }

    #[test]
    fn rejects_attribute_count_bombs_without_quadratic_duplicate_scans() {
        let mut one_element = String::from("<item");
        for index in 0..=MAX_XML_ATTRIBUTES_PER_ELEMENT {
            one_element.push_str(&format!(" a{index}=\"x\""));
        }
        one_element.push_str("/>");
        assert!(parse(one_element.as_bytes()).is_none());

        let mut document = String::from("<root>");
        let child_count = MAX_XML_ATTRIBUTES / MAX_XML_ATTRIBUTES_PER_ELEMENT + 1;
        for child in 0..child_count {
            document.push_str("<item");
            for index in 0..MAX_XML_ATTRIBUTES_PER_ELEMENT {
                document.push_str(&format!(" a{child}_{index}=\"x\""));
            }
            document.push_str("/>");
        }
        document.push_str("</root>");
        assert!(parse(document.as_bytes()).is_none());
    }

    #[test]
    fn rejects_excessive_stored_attribute_bytes() {
        let value = "x".repeat(MAX_XML_STORED_ATTRIBUTE_BYTES + 1);
        let document = format!("<item value=\"{value}\"/>");
        assert!(parse(document.as_bytes()).is_none());
    }

    #[test]
    fn parse_cdata_section() {
        let xml = b"<data><![CDATA[<not>xml</not>]]></data>";
        let node = parse(xml).unwrap();
        assert_eq!(node.text, "<not>xml</not>");
    }

    #[test]
    fn decode_xml_entities() {
        assert_eq!(decode_entities("&amp;&lt;&gt;&quot;&apos;"), "&<>\"'");
        assert_eq!(decode_entities("&#65;&#x42;"), "AB");
    }

    #[test]
    fn parse_with_prolog_and_comments() {
        let xml = b"<?xml version=\"1.0\"?><!-- comment --><root>text</root>";
        let node = parse(xml).unwrap();
        assert_eq!(node.tag, "root");
        assert_eq!(node.text, "text");
    }

    #[test]
    fn parses_utf8_text_without_corruption_or_panics() {
        let node = parse("<root>España 🚀</root>".as_bytes()).unwrap();
        assert_eq!(node.text, "España 🚀");
    }

    #[test]
    fn parses_namespaced_children_by_local_name() {
        let node = parse(b"<atom:feed><atom:title>News</atom:title></atom:feed>").unwrap();
        assert_eq!(node.local_name(), "feed");
        assert_eq!(node.child("title").unwrap().text, "News");
    }

    #[test]
    fn rejects_mismatched_unclosed_and_trailing_markup() {
        assert!(parse(b"<root><child></root></child>").is_none());
        assert!(parse(b"<root>").is_none());
        assert!(parse(b"<root/>junk").is_none());
        assert!(parse(b"<root/><!DOCTYPE second>").is_none());
        assert!(parse(b"<bad&name/>").is_none());
        assert!(parse(b"<root attr=\"bad<value\"/>").is_none());
    }

    #[test]
    fn preserves_incomplete_and_unknown_entities() {
        assert_eq!(decode_entities("one & two &custom;"), "one & two &custom;");
        assert_eq!(decode_entities("trailing &"), "trailing &");

        let hostile = format!("{};", "&".repeat(20_000));
        assert_eq!(decode_entities(&hostile), hostile);
    }

    #[test]
    fn parses_mixed_content_in_reading_order() {
        let node = parse(b"<title>One <b>bold</b> title</title>").unwrap();
        assert_eq!(node.text, "One bold title");
    }

    #[test]
    fn child_and_children_by_tag() {
        let xml = b"<root><a>1</a><b>2</b><a>3</a></root>";
        let node = parse(xml).unwrap();
        assert_eq!(node.child("b").unwrap().text, "2");
        let a_nodes = node.children_by_tag("a");
        assert_eq!(a_nodes.len(), 2);
        assert_eq!(a_nodes[0].text, "1");
        assert_eq!(a_nodes[1].text, "3");
    }

    #[test]
    fn parse_rss_fragment() {
        let xml = br#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Test Feed</title>
    <item>
      <title>Episode 1</title>
      <link>http://example.com/1</link>
      <guid>guid-001</guid>
      <enclosure url="http://example.com/1.torrent" type="application/x-bittorrent"/>
    </item>
  </channel>
</rss>"#;
        let root = parse(xml).unwrap();
        assert_eq!(root.tag, "rss");
        let channel = root.child("channel").unwrap();
        assert_eq!(channel.child("title").unwrap().text, "Test Feed");
        let item = channel.child("item").unwrap();
        assert_eq!(item.child("title").unwrap().text, "Episode 1");
        let enc = item.child("enclosure").unwrap();
        assert_eq!(enc.attr("url"), Some("http://example.com/1.torrent"));
    }
}
