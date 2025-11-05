use crossterm::cursor::MoveUp;
use crossterm::execute;
use crossterm::terminal::Clear;
use forge_spinner::SpinnerManager;
use termimad::crossterm::style::Attribute;

use crate::md::render::MarkdownRenderer;

pub struct MarkdownWriter<W> {
    buffer: String,
    renderer: MarkdownRenderer,
    previous_rendered: String,
    writer: W,
    last_was_dimmed: bool,
    // Micro-batching state to coalesce tiny chunks
    pending_bytes: usize,
    last_stream_at: Option<Instant>,
    // Config: render if pending >= threshold or window elapsed
    coalesce_bytes: usize,
    coalesce_window: Duration,
}

impl<W> MarkdownWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            buffer: String::new(),
            renderer: MarkdownRenderer::default(),
            previous_rendered: String::new(),
            writer,
            last_was_dimmed: false,
            pending_bytes: 0,
            last_stream_at: None,
            // Reasonable defaults: ~80 bytes or 60ms window
            coalesce_bytes: 200,
            coalesce_window: Duration::from_millis(200),
        }
    }
}
use std::thread;
use std::time::{Duration, Instant};

impl<W: std::io::Write> MarkdownWriter<W> {
    #[cfg(test)]
    fn with_renderer(mut self, renderer: MarkdownRenderer) -> Self {
        self.renderer = renderer;
        self
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.previous_rendered.clear();
        self.pending_bytes = 0;
        self.last_stream_at = None;
    }

    pub fn add_chunk(&mut self, chunk: &str, spn: &mut SpinnerManager) {
        // If switching from dimmed -> normal, flush any pending dimmed content first
        if self.last_was_dimmed {
            if self.pending_bytes > 0 {
                let rendered = self.renderer.render(&self.buffer, Some(Attribute::Dim));
                self.stream(&rendered, spn);
            }
            self.reset();
        }
        self.buffer.push_str(chunk);
        self.pending_bytes = self.pending_bytes.saturating_add(chunk.len());
        self.maybe_stream(None, spn);
        self.last_was_dimmed = false;
    }

    pub fn add_chunk_dimmed(&mut self, chunk: &str, spn: &mut SpinnerManager) {
        // If switching from normal -> dimmed, flush any pending normal content first
        if !self.last_was_dimmed {
            if self.pending_bytes > 0 {
                let rendered = self.renderer.render(&self.buffer, None);
                self.stream(&rendered, spn);
            }
            self.reset();
        }
        self.buffer.push_str(chunk);
        self.pending_bytes = self.pending_bytes.saturating_add(chunk.len());
        self.maybe_stream(Some(Attribute::Dim), spn);
        self.last_was_dimmed = true;
    }

    /// Flush any pending buffered updates immediately using the current mode
    pub fn flush(&mut self, spn: &mut SpinnerManager) {
        if self.pending_bytes == 0 {
            return;
        }
        let attr = if self.last_was_dimmed {
            Some(Attribute::Dim)
        } else {
            None
        };
        let rendered = self.renderer.render(&self.buffer, attr);
        self.stream(&rendered, spn);
        self.pending_bytes = 0;
        self.last_stream_at = Some(Instant::now());
    }

    // Decide whether to render based on pending size or debounce window
    fn maybe_stream(&mut self, attr: Option<Attribute>, spn: &mut SpinnerManager) {
        let should_stream = match self.last_stream_at {
            None => true, // Always render the first time
            Some(ts) => {
                self.pending_bytes >= self.coalesce_bytes || ts.elapsed() >= self.coalesce_window
            }
        };

        if should_stream {
            let rendered = self.renderer.render(&self.buffer, attr);
            self.stream(&rendered, spn);
            self.pending_bytes = 0;
            self.last_stream_at = Some(Instant::now());
        }
    }

    fn stream(&mut self, content: &str, spn: &mut SpinnerManager) {
        let rendered_lines: Vec<&str> = content.lines().collect();
        let lines_new: Vec<&str> = rendered_lines;
        let lines_prev: Vec<String> = self
            .previous_rendered
            .lines()
            .map(|s| s.to_string())
            .collect();

        // Hide the cursor
        write!(self.writer, "\x1b[?25l");
        // thread::sleep(Duration::from_millis(200));
        spn.suspend(|| {
            let common = lines_prev
                .iter()
                .map(|s| s.as_str())
                .zip(&lines_new)
                .take_while(|(p, n)| p == *n)
                .count();

            let lines_to_update = self.renderer.height;
            let mut skip = 0;
            let up_lines = lines_prev.len() - common;

            if up_lines > lines_to_update {
                skip = up_lines - lines_to_update;
            }
            let mut up_lines = (lines_prev.len() - common) - skip + 1;
            if up_lines > 0 {
                execute!(self.writer, MoveUp(up_lines as u16)).unwrap();
            }
            /*execute!(
                self.writer,
                Clear(crossterm::terminal::ClearType::FromCursorDown)
            )
            .unwrap();*/
            for line in lines_new[common + skip..].iter() {
                writeln!(self.writer, "{}", line).unwrap();
            }
            writeln!(self.writer).unwrap();
            self.writer.flush().unwrap();
            self.previous_rendered = content.to_string();
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use pretty_assertions::assert_eq;
    use strip_ansi_escapes::strip_str;

    use super::*;

    #[test]
    fn test_markdown_writer_basic_incremental_update() {
        let mut output = Vec::new();
        let mut spn = SpinnerManager::new();
        let previous_rendered = {
            let mut writer = MarkdownWriter::new(Box::new(Cursor::new(&mut output)));
            writer.stream("Line 1\nLine 2\nLine 3", &mut spn);
            writer.previous_rendered.clone()
        };
        assert_eq!(previous_rendered, "Line 1\nLine 2\nLine 3");
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Line 1"));
        assert!(output_str.contains("Line 2"));
        assert!(output_str.contains("Line 3"));
    }

    #[test]
    fn test_markdown_writer_full_clear_with_height_cap() {
        let renderer = MarkdownRenderer::new(80, 2);
        let mut output = Vec::new();
        let mut spn = SpinnerManager::new();
        {
            let mut writer = MarkdownWriter::new(Cursor::new(&mut output)).with_renderer(renderer);
            writer.previous_rendered = "Old 1\nOld 2\nOld 3\nOld 4\nOld 5".to_string();
            writer.stream("new 1\nnew 2\nnew3\nnew 4\n new 5\n new6", &mut spn);
        }
        let output_str = String::from_utf8(output).unwrap();
        // common=0, up_lines=5, height=2, skip=3, up_lines=2, print \x1b[2A \x1b[0J
        // New\n (take 2, but only 1 line) + 1 (for spinner is skips one line)
        assert!(output_str.contains("\x1b[3A"));
    }

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
        // Should have two reset codes for two code blocks
        let reset_count = actual.matches("\x1b[0m").count();
        assert_eq!(reset_count, 2);
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
    fn test_markdown_writer_long_text_chunk_by_chunk() {
        let mut fixture = MarkdownWriter::new(Box::new(std::io::sink()));
        let mut spn = SpinnerManager::new();

        let long_text = r#"# Header

This is a long paragraph with multiple sentences. It contains various types of content including some code examples.

```rust
fn main() {
    println!("Hello, world!");
    let x = 42;
    println!("The answer is {}", x);
}
```

And some more text after the code block."#;

        // Split into chunks and add with spaces
        let chunks = long_text.split_whitespace().collect::<Vec<_>>();
        for chunk in chunks {
            fixture.add_chunk(&format!("{} ", chunk), &mut spn);
        }

        assert!(fixture.buffer.contains("Header"));
        assert!(fixture.buffer.contains("println!"));
        assert!(fixture.buffer.contains("Hello, world!"));
        assert!(fixture.buffer.contains("more text"));
    }
}
