use derive_setters::Setters;
use lazy_regex::regex;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{LinesWithEndings, as_24_bit_terminal_escaped};
use termimad::crossterm::style::{Attribute, Color};
use termimad::crossterm::terminal;
use termimad::{Alignment, CompoundStyle, LineStyle, MadSkin};

#[derive(Debug)]
pub enum Segment {
    Text(String),
    Code(String),
}

#[derive(Setters)]
pub struct MarkdownRenderer {
    pub ss: SyntaxSet,
    pub theme: syntect::highlighting::Theme,
    pub width: usize,
    pub height: usize,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        let (width, height) = terminal::size().unwrap_or((80, 24));

        Self::new(
            (width as usize).saturating_sub(1),
            (height as usize).saturating_sub(1),
        )
    }
}

impl MarkdownRenderer {
    pub fn new(width: usize, height: usize) -> Self {
        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let theme = ts.themes["Solarized (dark)"].clone();

        Self { ss, theme, width, height }
    }

    pub fn render(&self, content: &str, attr: Option<Attribute>) -> String {
        let skin = create_skin(attr);
        let segments = self.render_markdown(content);
        let mut result = String::new();
        for segment in segments {
            match segment {
                Segment::Text(t) => {
                    let rendered = skin.text(&t, Some(self.width));
                    result.push_str(&rendered.to_string());
                }
                Segment::Code(c) => {
                    result.push_str(&c);
                }
            }
        }

        // Trimming of visible trailing whitespace per line (To trim termimad's extra whitespaces),
        // then wrap at the terminal width to prevent overflow.

        let cleaned = result
            .lines()
            .map(|line| rtrim_visible_preserve_sgr(line))
            .collect::<Vec<_>>()
            .join("\n");

        wrap_ansi_simple(&cleaned, self.width)
    }

    fn render_markdown(&self, text: &str) -> Vec<Segment> {
        // Match fenced code blocks similar to markdown_renderer::renderer
        let re = regex!(r"(?ms)^```(\w+)?\n(.*?)(^```|\z)");
        let mut segments = vec![];
        let mut last_end = 0;

        for cap in re.captures_iter(text) {
            let start = cap.get(0).unwrap().start();
            if start > last_end {
                segments.push(Segment::Text(text[last_end..start].to_string()));
            }
            let lang = cap.get(1).map(|m| m.as_str()).unwrap_or("txt");
            let code = cap.get(2).unwrap().as_str();

            let wrapped_code = wrap_code_simple(code, self.width);
            let syntax = self
                .ss
                .find_syntax_by_token(lang)
                .unwrap_or_else(|| self.ss.find_syntax_plain_text());

            let mut h = HighlightLines::new(syntax, &self.theme);
            let mut highlighted = String::from("\n");

            for line in LinesWithEndings::from(&wrapped_code) {
                let ranges: Vec<(syntect::highlighting::Style, &str)> =
                    h.highlight_line(line, &self.ss).unwrap();
                highlighted.push_str(&as_24_bit_terminal_escaped(&ranges[..], false));
            }

            highlighted.push_str("\x1b[0m");
            segments.push(Segment::Code(highlighted));
            last_end = cap.get(0).unwrap().end();
        }
        if last_end < text.len() {
            segments.push(Segment::Text(text[last_end..].to_string()));
        }
        segments
    }
}

// ANSI SGR parsing and wrapping utilities adapted from markdown_renderer
#[derive(Clone, Copy)]
struct SgrSeg {
    is_sgr: bool,
    start: usize,
    end: usize,
}

fn parse_sgr_segments(s: &str) -> Vec<SgrSeg> {
    let b = s.as_bytes();
    let mut segs = Vec::new();
    let mut i = 0usize;
    let mut text_start = 0usize;
    while i < b.len() {
        if b[i] == 0x1B && i + 1 < b.len() && b[i + 1] as char == '[' {
            // find 'm' terminator of SGR sequence
            let mut j = i + 2;
            while j < b.len() && b[j] as char != 'm' {
                j += 1;
            }
            if j < b.len() {
                if text_start < i {
                    segs.push(SgrSeg { is_sgr: false, start: text_start, end: i });
                }
                let end = j + 1; // include 'm'
                segs.push(SgrSeg { is_sgr: true, start: i, end });
                i = end;
                text_start = i;
                continue;
            }
        }
        i += 1;
    }
    if text_start < s.len() {
        segs.push(SgrSeg { is_sgr: false, start: text_start, end: s.len() });
    }
    segs
}

// Trim trailing visible spaces/tabs while preserving trailing SGR sequences
fn rtrim_visible_preserve_sgr(s: &str) -> String {
    let b = s.as_bytes();
    let segs = parse_sgr_segments(s);
    // Find the cut point (last non-space/tab in text segments)
    let mut cut: Option<usize> = None;
    for seg in segs.iter().rev() {
        if seg.is_sgr {
            continue;
        }
        let mut j = seg.end;
        while j > seg.start {
            let ch = b[j - 1];
            if ch == b' ' || ch == b'\t' {
                j -= 1;
            } else {
                cut = Some(j);
                break;
            }
        }
        if cut.is_some() {
            break;
        }
    }
    let Some(cut) = cut else { return String::new() };

    // Rebuild: include all SGR segments, and text only up to the cut
    let mut out = String::with_capacity(s.len());
    for seg in segs {
        if seg.is_sgr {
            out.push_str(&s[seg.start..seg.end]);
        } else if seg.end <= cut {
            out.push_str(&s[seg.start..seg.end]);
        } else if seg.start < cut {
            out.push_str(&s[seg.start..cut]);
        }
    }
    out
}

// Simple ANSI-aware hard wrapper. Counts only visible columns (SGR zero-width).
fn wrap_ansi_simple(s: &str, width: usize) -> String {
    if width == 0 {
        return s.to_string();
    }
    let segs = parse_sgr_segments(s);
    let mut out = String::with_capacity(s.len() + s.len() / (width.max(1)) + 8);
    let mut col = 0usize;
    for seg in segs {
        if seg.is_sgr {
            out.push_str(&s[seg.start..seg.end]);
            continue;
        }
        let text = &s[seg.start..seg.end];
        for ch in text.chars() {
            if ch == '\n' {
                out.push('\n');
                col = 0;
                continue;
            }
            if ch == '\r' {
                out.push('\r');
                continue;
            }
            if col >= width {
                out.push('\n');
                col = 0;
            }
            out.push(ch);
            col += 1;
        }
    }
    out
}

// Pre-wrap raw code lines at fixed width (no ANSI), like markdown_renderer
fn wrap_code_simple(code: &str, width: usize) -> String {
    if width == 0 {
        return code.to_string();
    }
    let mut result = String::new();
    for line in code.lines() {
        if line.len() <= width {
            result.push_str(line);
            result.push('\n');
        } else {
            let mut start = 0;
            let bytes = line.as_bytes();
            while start < bytes.len() {
                let end = (start + width).min(bytes.len());
                // Safety: slicing at byte indices; assumes ASCII code input
                result.push_str(&line[start..end]);
                result.push('\n');
                start = end;
            }
        }
    }
    result
}

fn create_skin(attr: Option<Attribute>) -> MadSkin {
    let mut skin = MadSkin::default();

    // Inline Code
    let style = CompoundStyle::new(Some(Color::Cyan), None, Attribute::Bold.into());
    skin.inline_code = style;

    // Code Blocks
    let codeblock_style = CompoundStyle::new(None, None, Default::default());
    skin.code_block = LineStyle::new(codeblock_style, Default::default());

    // Strikethrough
    let mut style = CompoundStyle::with_attr(Attribute::CrossedOut);
    style.add_attr(Attribute::Dim);
    skin.strikeout = style;

    // Headings
    let mut style = LineStyle::default();
    style.add_attr(Attribute::Bold);
    style.set_fg(Color::Green);

    let mut h1 = style.clone();
    h1.align = Alignment::Center;
    skin.headers = [
        h1,
        style.clone(),
        style.clone(),
        style.clone(),
        style.clone(),
        style.clone(),
        style.clone(),
        style.clone(),
    ];

    // Custom Attribute
    if let Some(attr) = attr {
        skin.paragraph.add_attr(attr);
        skin.inline_code.add_attr(attr);
        skin.code_block.compound_style.add_attr(attr);
        skin.strikeout.add_attr(attr);
    }

    skin
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use strip_ansi_escapes::strip_str;

    use super::*;

    #[test]
    fn test_render_plain_text() {
        let fixture = MarkdownRenderer::new(80, 24);
        let input = "This is plain text.\n\nWith multiple lines.";
        let actual = fixture.render(input, None);
        let clean_actual = strip_str(&actual);
        assert!(clean_actual.contains("This is plain text."));
        assert!(clean_actual.contains("With multiple lines."));
    }

    #[test]
    fn test_render_multiple_code_blocks() {
        let fixture = MarkdownRenderer::new(80, 24);
        let input = "Text 1\n\n```\ncode1\n```\n\nText 2\n\n```\ncode2\n```\n\nText 3";
        let actual = fixture.render(input, None);
        let clean_actual = strip_str(&actual);
        assert!(clean_actual.contains("Text 1"));
        assert!(clean_actual.contains("code1"));
        assert!(clean_actual.contains("Text 2"));
        assert!(clean_actual.contains("code2"));
        assert!(clean_actual.contains("Text 3"));
    }

    #[test]
    fn test_render_unclosed_code_block() {
        let fixture = MarkdownRenderer::new(80, 24);
        let input = "Text\n\n```\nunclosed code";
        let actual = fixture.render(input, None);
        let clean_actual = strip_str(&actual);
        assert!(clean_actual.contains("Text"));
        assert!(clean_actual.contains("unclosed code"));
        assert!(actual.contains("\x1b[0m"));
    }

    #[test]
    fn test_segments_plain_text() {
        let fixture = MarkdownRenderer::new(80, 24);
        let input = "This is plain text.\n\nWith multiple lines.";
        let segments = fixture.render_markdown(input);
        assert_eq!(segments.len(), 1);
        assert!(matches!(segments[0], Segment::Text(ref t) if t == input));
    }

    #[test]
    fn test_segments_single_code_block_middle() {
        let fixture = MarkdownRenderer::new(80, 24);
        let input = "Before code.\n\n```\nfn main() {}\n```\n\nAfter code.";
        let segments = fixture.render_markdown(input);
        assert_eq!(segments.len(), 3);
        assert!(matches!(segments[0], Segment::Text(ref t) if t.contains("Before code.")));
        assert!(
            matches!(segments[1], Segment::Code(ref c) if strip_str(c).contains("fn main() {}"))
        );
        assert!(matches!(segments[2], Segment::Text(ref t) if t.contains("After code.")));
    }

    #[test]
    fn test_segments_multiple_code_blocks() {
        let fixture = MarkdownRenderer::new(80, 24);
        let input = "Text 1\n\n```\ncode1\n```\n\nText 2\n\n```\ncode2\n```\n\nText 3";
        let segments = fixture.render_markdown(input);
        assert_eq!(segments.len(), 5); // Text, Code, Text, Code, Text
        let code_count = segments
            .iter()
            .filter(|s| matches!(s, Segment::Code(_)))
            .count();
        assert_eq!(code_count, 2);
        let text_count = segments
            .iter()
            .filter(|s| matches!(s, Segment::Text(_)))
            .count();
        assert_eq!(text_count, 3);
    }
}
