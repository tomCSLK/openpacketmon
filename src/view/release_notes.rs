//! The release notes of a version as GitHub keeps them (Markdown), cut into what the settings
//! show (`.relnotes`): headings, paragraphs and list items with bold words, code and links.
//! Only this little part of Markdown is read; everything else stays text.

/// A piece of a line
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Span {
    Text(String),
    Bold(String),
    Code(String),
    /// The text of a link and where it leads
    Link(String, String),
}

/// A block of the notes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading(Vec<Span>),
    Paragraph(Vec<Span>),
    /// An entry of a list (`-`, `*` or `+`)
    Item(Vec<Span>),
}

/// Most blocks shown; the notes come from the network
const MAX_BLOCKS: usize = 400;

/// Cuts the notes into blocks
pub fn blocks(notes: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph = String::new();
    let mut item: Option<String> = None;
    let flush = |blocks: &mut Vec<Block>, paragraph: &mut String, item: &mut Option<String>| {
        if let Some(text) = item.take() {
            blocks.push(Block::Item(spans(&text)));
        }
        if !paragraph.trim().is_empty() {
            blocks.push(Block::Paragraph(spans(paragraph.trim())));
        }
        paragraph.clear();
    };
    for line in notes.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush(&mut blocks, &mut paragraph, &mut item);
            continue;
        }
        if let Some(heading) = heading(trimmed) {
            flush(&mut blocks, &mut paragraph, &mut item);
            blocks.push(Block::Heading(spans(heading)));
            continue;
        }
        if let Some(text) = list_item(trimmed) {
            flush(&mut blocks, &mut paragraph, &mut item);
            item = Some(text.to_owned());
            continue;
        }
        // a line that goes on with the item or paragraph above
        if let Some(text) = &mut item {
            text.push(' ');
            text.push_str(trimmed);
        } else {
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(trimmed);
        }
    }
    flush(&mut blocks, &mut paragraph, &mut item);
    blocks.truncate(MAX_BLOCKS);
    blocks
}

fn heading(line: &str) -> Option<&str> {
    let text = line.trim_start_matches('#');
    let level = line.len() - text.len();
    ((1..=6).contains(&level) && text.starts_with(' ')).then(|| text.trim())
}

fn list_item(line: &str) -> Option<&str> {
    ["- ", "* ", "+ "]
        .iter()
        .find_map(|mark| line.strip_prefix(mark))
        .map(str::trim)
}

/// The pieces of a line: `**bold**`, `` `code` `` and `[text](url)`; anything else is text
pub fn spans(line: &str) -> Vec<Span> {
    let mut out = Vec::new();
    let mut text = String::new();
    let mut rest = line;
    while let Some(c) = rest.chars().next() {
        let found = if let Some(after) = rest.strip_prefix("**") {
            after
                .find("**")
                .filter(|&end| end > 0)
                .map(|end| (Span::Bold(plain(&after[..end])), 4 + end))
        } else if let Some(after) = rest.strip_prefix('`') {
            after
                .find('`')
                .filter(|&end| end > 0)
                .map(|end| (Span::Code(after[..end].to_owned()), 2 + end))
        } else if c == '[' {
            link(rest)
        } else {
            None
        };
        if let Some((span, used)) = found {
            if !text.is_empty() {
                out.push(Span::Text(std::mem::take(&mut text)));
            }
            out.push(span);
            rest = &rest[used..];
        } else {
            text.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    if !text.is_empty() {
        out.push(Span::Text(text));
    }
    out
}

/// `[text](url)` at the start of `rest`, and how many bytes it takes
fn link(rest: &str) -> Option<(Span, usize)> {
    let close = rest.find("](")?;
    let end = rest[close + 2..].find(')')?;
    let text = &rest[1..close];
    let url = &rest[close + 2..close + 2 + end];
    (!text.is_empty() && !text.contains('[')).then(|| {
        (
            Span::Link(plain(text), url.trim().to_owned()),
            close + 3 + end,
        )
    })
}

/// A text without the marks of Markdown in it
fn plain(text: &str) -> String {
    text.replace("**", "").replace('`', "")
}

/// Whether a link of the notes may be opened: only pages on the web
pub fn openable(url: &str) -> bool {
    url.starts_with("https://") && !url.chars().any(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_are_cut_into_headings_paragraphs_and_items() {
        let notes = "New interface.\n\n### Changed\n- Header bar instead of the sidebar;\n  \
                     the recording stays visible\n* Fira Sans\n\nSee the [changelog](https://\
                     github.com/x/y/blob/main/CHANGELOG.md) and `openpacketmon mcp`, **off** by \
                     default.";
        assert_eq!(
            blocks(notes),
            [
                Block::Paragraph(vec![Span::Text("New interface.".into())]),
                Block::Heading(vec![Span::Text("Changed".into())]),
                Block::Item(vec![Span::Text(
                    "Header bar instead of the sidebar; the recording stays visible".into()
                )]),
                Block::Item(vec![Span::Text("Fira Sans".into())]),
                Block::Paragraph(vec![
                    Span::Text("See the ".into()),
                    Span::Link(
                        "changelog".into(),
                        "https://github.com/x/y/blob/main/CHANGELOG.md".into()
                    ),
                    Span::Text(" and ".into()),
                    Span::Code("openpacketmon mcp".into()),
                    Span::Text(", ".into()),
                    Span::Bold("off".into()),
                    Span::Text(" by default.".into()),
                ]),
            ]
        );
    }

    #[test]
    fn marks_without_their_end_stay_text() {
        assert_eq!(
            spans("a ** b ` c [d] (e) #1"),
            [Span::Text("a ** b ` c [d] (e) #1".into())]
        );
        assert_eq!(blocks("#hashtag"), [Block::Paragraph(spans("#hashtag"))]);
        assert!(openable("https://github.com/tomCSLK/openpacketmon"));
        assert!(!openable("file:///etc/passwd"));
        assert!(!openable("javascript:alert(1)"));
    }
}
