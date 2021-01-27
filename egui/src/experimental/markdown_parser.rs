//! A parser for a VERY (and intentionally so) strict and limited sub-set of Markdown.
//!
//! WARNING: the parsed dialect is subject to change.
//!
//! Does not depend on anything else in egui (could perhaps be its own crate if it grows).

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MarkdownItem<'a> {
    Newline,
    Separator,
    BulletPoint(usize),
    Body(&'a str),
    Heading(&'a str),
    Emphasis(&'a str),
    InlineCode(&'a str),
    Hyperlink(&'a str, &'a str),
}

pub struct MarkdownParser<'a> {
    s: &'a str,
    start_of_line: bool,
}

impl<'a> MarkdownParser<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            s,
            start_of_line: true,
        }
    }
}

impl<'a> Iterator for MarkdownParser<'a> {
    type Item = MarkdownItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let Self { s, start_of_line } = self;

        if s.is_empty() {
            return None;
        }

        //
        if s.starts_with("---\n") {
            *s = &s[4..];
            *start_of_line = true;
            return Some(MarkdownItem::Separator);
        }

        //
        if s.starts_with('\n') {
            *s = &s[1..];
            *start_of_line = true;
            return Some(MarkdownItem::Newline);
        }

        // # Heading
        if *start_of_line && s.starts_with("# ") {
            *s = &s[2..];
            *start_of_line = false;
            let end = s.find('\n').unwrap_or_else(|| s.len());
            let item = MarkdownItem::Heading(&s[..end]);
            *s = &s[end..];
            return Some(item);
        }

        // Ugly way to parse bullet points with indentation.
        // TODO: parse leading spaces separately as `MarkdownItem::Indentation`.
        for bullet in &["* ", " * ", "  * ", "   * ", "    * "] {
            // * bullet point
            if *start_of_line && s.starts_with(bullet) {
                *s = &s[bullet.len()..];
                *start_of_line = false;
                return Some(MarkdownItem::BulletPoint(bullet.len() - 2));
            }
        }

        // `code`
        if s.starts_with('`') {
            *s = &s[1..];
            *start_of_line = false;
            if let Some(end) = s.find('`') {
                let item = MarkdownItem::InlineCode(&s[..end]);
                *s = &s[end + 1..];
                return Some(item);
            } else {
                let end = s.len();
                let item = MarkdownItem::InlineCode(&s[..end]);
                *s = &s[end..];
                return Some(item);
            }
        }

        // *emphasis*
        if s.starts_with('*') {
            *s = &s[1..];
            *start_of_line = false;
            if let Some(end) = s.find('*') {
                let item = MarkdownItem::Emphasis(&s[..end]);
                *s = &s[end + 1..];
                return Some(item);
            } else {
                let end = s.len();
                let item = MarkdownItem::Emphasis(&s[..end]);
                *s = &s[end..];
                return Some(item);
            }
        }

        // [text](url)
        if s.starts_with('[') {
            if let Some(bracket_end) = s.find(']') {
                let text = &s[1..bracket_end];
                if s[bracket_end + 1..].starts_with('(') {
                    if let Some(parens_end) = s[bracket_end + 2..].find(')') {
                        let parens_end = bracket_end + 2 + parens_end;
                        let url = &s[bracket_end + 2..parens_end];
                        *s = &s[parens_end + 1..];
                        *start_of_line = false;
                        return Some(MarkdownItem::Hyperlink(text, url));
                    }
                }
            }
        }

        let end = s[1..]
            .find(&['#', '*', '`', '[', '\n'][..])
            .map(|i| i + 1)
            .unwrap_or_else(|| s.len());
        let item = MarkdownItem::Body(&s[..end]);
        *s = &s[end..];
        *start_of_line = false;
        Some(item)
    }
}

#[test]
fn test_markdown() {
    let parts: Vec<_> = MarkdownParser::new("# Hello\nworld `of` *fun* [link](url)").collect();
    assert_eq!(
        parts,
        vec![
            MarkdownItem::Heading("Hello"),
            MarkdownItem::Newline,
            MarkdownItem::Body("world "),
            MarkdownItem::InlineCode("of"),
            MarkdownItem::Body(" "),
            MarkdownItem::Emphasis("fun"),
            MarkdownItem::Body(" "),
            MarkdownItem::Hyperlink("link", "url")
        ]
    );
}
