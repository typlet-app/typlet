// SPDX-License-Identifier: GPL-3.0-or-later
//! Turns plain text, Markdown or HTML with espanso placeholders into a
//! Typlet [`Document`].
//!
//! The converters are deliberately small: they understand the formatting a
//! Typlet snippet can hold (bold, italic, underline, links, line breaks,
//! paragraphs, bullet and numbered lists) and drop the rest, setting
//! [`DocBuilder::simplified`].

use typlet_core::{Block, Document, Inline, Marks, SnippetId};

use crate::SkipReason;

/// A placeholder found in the text, already resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Placeholder {
    Date {
        format: String,
        offset_days: i64,
    },
    Clipboard,
    Text(String),
    Field(String),
    /// Another match, by a placeholder id that is linked after the import.
    Snippet(SnippetId),
}

/// Resolves `{{name}}` references.
pub(crate) type Resolver<'a> = &'a dyn Fn(&str) -> Result<Placeholder, SkipReason>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListKind {
    Bullet,
    Ordered,
}

/// Collects blocks and inlines.
pub(crate) struct DocBuilder<'a> {
    blocks: Vec<Block>,
    paragraph: Vec<Inline>,
    list: Option<(ListKind, Vec<Vec<Inline>>)>,
    resolve: Resolver<'a>,
    /// `[[field]]` placeholders are recognised (espanso forms).
    form: bool,
    cursor_used: bool,
    /// Some formatting could not be represented.
    pub(crate) simplified: bool,
}

impl<'a> DocBuilder<'a> {
    pub(crate) fn new(resolve: Resolver<'a>, form: bool) -> Self {
        Self {
            blocks: Vec::new(),
            paragraph: Vec::new(),
            list: None,
            resolve,
            form,
            cursor_used: false,
            simplified: false,
        }
    }

    fn current(&mut self) -> &mut Vec<Inline> {
        match &mut self.list {
            Some((_, items)) => {
                if items.is_empty() {
                    items.push(Vec::new());
                }
                let last = items.len() - 1;
                &mut items[last]
            }
            None => &mut self.paragraph,
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        let current = self.current();
        // Merge adjacent text runs with the same marks.
        if let (
            Some(Inline::Text { text, marks }),
            Inline::Text {
                text: new,
                marks: new_marks,
            },
        ) = (current.last_mut(), &inline)
            && marks == new_marks
        {
            text.push_str(new);
            return;
        }
        current.push(inline);
    }

    /// Adds text that may contain placeholders but no newlines.
    pub(crate) fn text(&mut self, text: &str, marks: &Marks) -> Result<(), SkipReason> {
        let mut rest = text;
        while !rest.is_empty() {
            let next = [
                rest.find("{{").map(|i| (i, 0)),
                rest.find("$|$").map(|i| (i, 1)),
                if self.form {
                    rest.find("[[").map(|i| (i, 2))
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            .min();
            let Some((start, kind)) = next else {
                self.plain(rest, marks);
                break;
            };
            self.plain(&rest[..start], marks);
            match kind {
                0 => {
                    let Some(end) = rest[start..].find("}}") else {
                        self.plain(&rest[start..], marks);
                        break;
                    };
                    let name = rest[start + 2..start + end].trim();
                    let placeholder = (self.resolve)(name)?;
                    self.placeholder(placeholder, marks);
                    rest = &rest[start + end + 2..];
                }
                1 => {
                    if !self.cursor_used {
                        self.cursor_used = true;
                        self.push_inline(Inline::Cursor);
                    }
                    rest = &rest[start + 3..];
                }
                _ => {
                    let Some(end) = rest[start..].find("]]") else {
                        self.plain(&rest[start..], marks);
                        break;
                    };
                    let name = rest[start + 2..start + end].trim().to_owned();
                    self.push_inline(Inline::Field { name });
                    rest = &rest[start + end + 2..];
                }
            }
        }
        Ok(())
    }

    fn plain(&mut self, text: &str, marks: &Marks) {
        if !text.is_empty() {
            self.push_inline(Inline::styled(text, marks.clone()));
        }
    }

    fn placeholder(&mut self, placeholder: Placeholder, marks: &Marks) {
        match placeholder {
            Placeholder::Date {
                format,
                offset_days,
            } => self.push_inline(Inline::DateTime {
                format,
                offset_days,
            }),
            Placeholder::Clipboard => self.push_inline(Inline::Clipboard),
            Placeholder::Text(text) => {
                for (i, line) in text.split('\n').enumerate() {
                    if i > 0 {
                        self.line_break();
                    }
                    self.plain(line, marks);
                }
            }
            Placeholder::Field(name) => self.push_inline(Inline::Field { name }),
            Placeholder::Snippet(id) => self.push_inline(Inline::Snippet { id }),
        }
    }

    pub(crate) fn line_break(&mut self) {
        self.current().push(Inline::LineBreak);
    }

    /// Ends the current paragraph. `keep_empty` keeps it even without
    /// content (an empty line in plain text).
    pub(crate) fn end_paragraph(&mut self, keep_empty: bool) {
        self.end_list();
        let content = std::mem::take(&mut self.paragraph);
        if keep_empty || !content.is_empty() {
            self.blocks.push(Block::Paragraph { content });
        }
    }

    fn start_list(&mut self, kind: ListKind) {
        if self.list.as_ref().is_some_and(|(k, _)| *k == kind) {
            return;
        }
        self.end_paragraph(false);
        self.list = Some((kind, Vec::new()));
    }

    fn start_item(&mut self) {
        if let Some((_, items)) = &mut self.list {
            items.push(Vec::new());
        }
    }

    fn end_list(&mut self) {
        if let Some((kind, items)) = self.list.take() {
            let items: Vec<Vec<Inline>> = items.into_iter().filter(|i| !i.is_empty()).collect();
            if !items.is_empty() {
                self.blocks.push(match kind {
                    ListKind::Bullet => Block::BulletList { items },
                    ListKind::Ordered => Block::OrderedList { items },
                });
            }
        }
    }

    pub(crate) fn finish(mut self) -> (Document, bool) {
        self.end_paragraph(false);
        // Trailing empty paragraphs come from a final newline in YAML.
        while matches!(self.blocks.last(), Some(Block::Paragraph { content }) if content.is_empty())
        {
            self.blocks.pop();
        }
        (
            Document {
                blocks: self.blocks,
            },
            self.simplified,
        )
    }
}

/// Converts plain text: every line becomes a paragraph.
pub(crate) fn from_plain(builder: &mut DocBuilder<'_>, text: &str) -> Result<(), SkipReason> {
    for line in text.replace("\r\n", "\n").split('\n') {
        builder.text(line, &Marks::default())?;
        builder.end_paragraph(true);
    }
    Ok(())
}

/// Converts the Markdown subset espanso users typically write.
pub(crate) fn from_markdown(builder: &mut DocBuilder<'_>, text: &str) -> Result<(), SkipReason> {
    let mut in_paragraph = false;
    for raw in text.replace("\r\n", "\n").split('\n') {
        let line = raw.trim_end();
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            builder.end_paragraph(false);
            in_paragraph = false;
            continue;
        }
        if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("+ "))
        {
            builder.start_list(ListKind::Bullet);
            builder.start_item();
            markdown_inline(builder, item)?;
            in_paragraph = false;
            continue;
        }
        if let Some(item) = ordered_item(trimmed) {
            builder.start_list(ListKind::Ordered);
            builder.start_item();
            markdown_inline(builder, item)?;
            in_paragraph = false;
            continue;
        }
        if builder.list.is_some() {
            builder.end_list();
        }
        let (content, heading) = match trimmed.trim_start_matches('#') {
            rest if rest.len() < trimmed.len() && rest.starts_with(' ') => {
                (rest.trim_start(), true)
            }
            _ => (trimmed, false),
        };
        if in_paragraph {
            builder.line_break();
        }
        if heading {
            builder.simplified = true;
            builder.end_paragraph(false);
            let marks = Marks {
                bold: true,
                ..Marks::default()
            };
            builder.text(content, &marks)?;
            builder.end_paragraph(false);
            in_paragraph = false;
        } else {
            markdown_inline(builder, content)?;
            in_paragraph = true;
        }
    }
    Ok(())
}

fn ordered_item(line: &str) -> Option<&str> {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let rest = &line[digits..];
    rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") "))
}

/// Bold (`**`, `__`), italic (`*`, `_`), links (`[text](url)`) and code
/// (shown as plain text).
fn markdown_inline(builder: &mut DocBuilder<'_>, text: &str) -> Result<(), SkipReason> {
    let mut marks = Marks::default();
    let mut buffer = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let flush = |builder: &mut DocBuilder<'_>, buffer: &mut String, marks: &Marks| {
        let result = builder.text(buffer, marks);
        buffer.clear();
        result
    };
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let prev_alnum = i > 0 && chars[i - 1].is_alphanumeric();
        if (c == '*' || c == '_') && next == Some(c) {
            flush(builder, &mut buffer, &marks)?;
            marks.bold = !marks.bold;
            i += 2;
            continue;
        }
        if c == '*'
            || (c == '_' && !prev_alnum && !marks.italic)
            || (c == '_' && marks.italic && !next.is_some_and(char::is_alphanumeric))
        {
            flush(builder, &mut buffer, &marks)?;
            marks.italic = !marks.italic;
            i += 1;
            continue;
        }
        if c == '`' {
            builder.simplified = true;
            i += 1;
            continue;
        }
        if c == '['
            && let Some((label, url, used)) = parse_link(&chars[i..])
        {
            flush(builder, &mut buffer, &marks)?;
            let link_marks = Marks {
                link: Some(url),
                ..marks.clone()
            };
            builder.text(&label, &link_marks)?;
            i += used;
            continue;
        }
        buffer.push(c);
        i += 1;
    }
    flush(builder, &mut buffer, &marks)
}

fn parse_link(chars: &[char]) -> Option<(String, String, usize)> {
    let close = chars.iter().position(|&c| c == ']')?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = chars[close + 2..].iter().position(|&c| c == ')')? + close + 2;
    let label: String = chars[1..close].iter().collect();
    let url: String = chars[close + 2..end].iter().collect();
    Some((label, url.trim().to_owned(), end + 1))
}

/// Converts the HTML subset found in snippets.
pub(crate) fn from_html(builder: &mut DocBuilder<'_>, html: &str) -> Result<(), SkipReason> {
    let mut marks_stack: Vec<(String, Marks)> = Vec::new();
    let mut marks = Marks::default();
    let mut rest = html;
    // Starts `true` so leading whitespace of a block is dropped.
    let mut last_was_space = true;

    while !rest.is_empty() {
        if let Some(tag_start) = rest.strip_prefix('<') {
            let Some(end) = tag_start.find('>') else {
                break;
            };
            let tag = &tag_start[..end];
            rest = &tag_start[end + 1..];
            let closing = tag.starts_with('/');
            let body = tag.trim_start_matches('/').trim_end_matches('/').trim();
            let name = body
                .split(|c: char| c.is_whitespace())
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            match (name.as_str(), closing) {
                ("b" | "strong" | "i" | "em" | "u" | "a", false) => {
                    marks_stack.push((name.clone(), marks.clone()));
                    match name.as_str() {
                        "b" | "strong" => marks.bold = true,
                        "i" | "em" => marks.italic = true,
                        "u" => marks.underline = true,
                        _ => marks.link = attribute(body, "href"),
                    }
                }
                ("b" | "strong" | "i" | "em" | "u" | "a", true) => {
                    if let Some(pos) = marks_stack.iter().rposition(|(n, _)| *n == name) {
                        marks = marks_stack[pos].1.clone();
                        marks_stack.truncate(pos);
                    }
                }
                ("br", _) => {
                    builder.line_break();
                    last_was_space = true;
                }
                ("p" | "div", _) => {
                    builder.end_paragraph(false);
                    last_was_space = true;
                }
                ("ul", false) => builder.start_list(ListKind::Bullet),
                ("ol", false) => builder.start_list(ListKind::Ordered),
                ("ul" | "ol", true) => builder.end_list(),
                ("li", false) => {
                    builder.start_item();
                    last_was_space = true;
                }
                ("li" | "span" | "html" | "body" | "meta", _) | ("", _) => {}
                _ => builder.simplified = true,
            }
            continue;
        }
        let end = rest.find('<').unwrap_or(rest.len());
        let text = decode_entities(&rest[..end]);
        rest = &rest[end..];
        // HTML collapses whitespace; a space belongs to the text before it.
        let mut out = String::new();
        for c in text.chars() {
            if c.is_whitespace() && c != '\u{a0}' {
                if !last_was_space {
                    out.push(' ');
                    last_was_space = true;
                }
            } else {
                last_was_space = false;
                out.push(if c == '\u{a0}' { ' ' } else { c });
            }
        }
        builder.text(&out, &marks)?;
    }
    Ok(())
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let start = lower.find(&format!("{name}="))? + name.len() + 1;
    let value = &tag[start..];
    let quote = value.chars().next()?;
    if quote == '"' || quote == '\'' {
        let end = value[1..].find(quote)?;
        Some(decode_entities(&value[1..=end]))
    } else {
        Some(
            value
                .split(|c: char| c.is_whitespace())
                .next()
                .unwrap_or_default()
                .to_owned(),
        )
    }
}

fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find(';').filter(|&e| e <= 8) else {
            out.push('&');
            rest = after;
            continue;
        };
        let entity = &after[..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            "nbsp" => Some('\u{a0}'),
            _ => entity
                .strip_prefix("#x")
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|dec| dec.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &after[end + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_vars(name: &str) -> Result<Placeholder, SkipReason> {
        match name {
            "today" => Ok(Placeholder::Date {
                format: "%d.%m.%Y".into(),
                offset_days: 0,
            }),
            "clipboard" => Ok(Placeholder::Clipboard),
            other => Err(SkipReason::UnsupportedVariable(other.into())),
        }
    }

    fn bold() -> Marks {
        Marks {
            bold: true,
            ..Marks::default()
        }
    }

    fn convert(f: fn(&mut DocBuilder<'_>, &str) -> Result<(), SkipReason>, text: &str) -> Document {
        let mut builder = DocBuilder::new(&no_vars, true);
        f(&mut builder, text).unwrap();
        builder.finish().0
    }

    #[test]
    fn plain_text_keeps_lines_and_placeholders() {
        let doc = convert(
            from_plain,
            "Hallo $|$\n\nam {{today}}: {{clipboard}} [[name]]\n",
        );
        assert_eq!(
            doc.blocks,
            vec![
                Block::Paragraph {
                    content: vec![Inline::text("Hallo "), Inline::Cursor]
                },
                Block::Paragraph { content: vec![] },
                Block::Paragraph {
                    content: vec![
                        Inline::text("am "),
                        Inline::DateTime {
                            format: "%d.%m.%Y".into(),
                            offset_days: 0
                        },
                        Inline::text(": "),
                        Inline::Clipboard,
                        Inline::text(" "),
                        Inline::Field {
                            name: "name".into()
                        },
                    ]
                },
            ]
        );
    }

    #[test]
    fn unknown_variables_are_reported() {
        let mut builder = DocBuilder::new(&no_vars, false);
        assert_eq!(
            from_plain(&mut builder, "{{output}}"),
            Err(SkipReason::UnsupportedVariable("output".into()))
        );
    }

    #[test]
    fn markdown_formatting_and_lists() {
        let doc = convert(
            from_markdown,
            "**Hi** _there_, see [docs](https://x.org)\nsecond line\n\n- one\n- two\n1. first",
        );
        assert_eq!(
            doc.blocks[0],
            Block::Paragraph {
                content: vec![
                    Inline::styled("Hi", bold()),
                    Inline::text(" "),
                    Inline::styled(
                        "there",
                        Marks {
                            italic: true,
                            ..Marks::default()
                        }
                    ),
                    Inline::text(", see "),
                    Inline::styled(
                        "docs",
                        Marks {
                            link: Some("https://x.org".into()),
                            ..Marks::default()
                        }
                    ),
                    Inline::LineBreak,
                    Inline::text("second line"),
                ]
            }
        );
        assert_eq!(
            doc.blocks[1],
            Block::BulletList {
                items: vec![vec![Inline::text("one")], vec![Inline::text("two")]]
            }
        );
        assert_eq!(
            doc.blocks[2],
            Block::OrderedList {
                items: vec![vec![Inline::text("first")]]
            }
        );
    }

    #[test]
    fn snake_case_words_are_not_italic() {
        let doc = convert(from_markdown, "file_name_here");
        assert_eq!(
            doc.blocks,
            vec![Block::Paragraph {
                content: vec![Inline::text("file_name_here")]
            }]
        );
    }

    #[test]
    fn html_formatting_lists_and_entities() {
        let mut builder = DocBuilder::new(&no_vars, false);
        from_html(
            &mut builder,
            "<p>Hello <b>World</b> &amp; <a href=\"https://x.org\">link</a><br>next</p>\n<ul><li>a</li><li><i>b</i></li></ul><h1>T</h1>",
        )
        .unwrap();
        let (doc, simplified) = builder.finish();
        assert!(simplified, "h1 is not supported");
        assert_eq!(
            doc.blocks[0],
            Block::Paragraph {
                content: vec![
                    Inline::text("Hello "),
                    Inline::styled("World", bold()),
                    Inline::text(" & "),
                    Inline::styled(
                        "link",
                        Marks {
                            link: Some("https://x.org".into()),
                            ..Marks::default()
                        }
                    ),
                    Inline::LineBreak,
                    Inline::text("next"),
                ]
            }
        );
        assert_eq!(
            doc.blocks[1],
            Block::BulletList {
                items: vec![
                    vec![Inline::text("a")],
                    vec![Inline::styled(
                        "b",
                        Marks {
                            italic: true,
                            ..Marks::default()
                        }
                    )]
                ]
            }
        );
        assert_eq!(
            decode_entities("&#228;&#xFC;&nbsp;&bogus; &"),
            "äü\u{a0}&bogus; &"
        );
    }
}
