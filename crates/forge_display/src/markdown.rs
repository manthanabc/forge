use std::io;

use lazy_regex::regex;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{LinesWithEndings, as_24_bit_terminal_escaped};
use termimad::MadSkin;

#[derive(Debug)]
pub enum Segment {
    Text(String),
    Code(String),
}

pub struct MarkdownRenderer {
    skin: MadSkin,
    ss: SyntaxSet,
    theme: syntect::highlighting::Theme,
    width: usize,
}

impl MarkdownRenderer {
    pub fn new(skin: MadSkin, width: usize) -> Self {
        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let theme = ts.themes["Solarized (dark)"].clone();
        Self { skin, ss, theme, width }
    }

    pub fn render(&self, content: &str) -> String {
        let segments = self.render_markdown(content);
        let mut result = String::new();
        for segment in segments {
            match segment {
                Segment::Text(t) => {
                    let rendered = self.skin.text(&t, Some(self.width));
                    result.push_str(&rendered.to_string());
                }
                Segment::Code(c) => {
                    result.push_str(&c);
                }
            }
        }
        result
    }

    fn render_markdown(&self, text: &str) -> Vec<Segment> {
        let re = regex!(r"(?s)```(\w+)?\n(.*?)(```|\z)");
        let mut segments = vec![];
        let mut last_end = 0;
        for cap in re.captures_iter(text) {
            let start = cap.get(0).unwrap().start();
            if start > last_end {
                segments.push(Segment::Text(text[last_end..start].to_string()));
            }
            let lang = cap.get(1).map(|m| m.as_str()).unwrap_or("txt");
            let ext = match lang {
                "rust" => "rs",
                "javascript" => "js",
                "python" => "py",
                _ => lang,
            };
            let code = cap.get(2).unwrap().as_str();
            let wrapped_code = Self::wrap_code(code, self.width);
            let syntax = self
                .ss
                .find_syntax_by_extension(ext)
                .unwrap_or_else(|| self.ss.find_syntax_plain_text());
            let mut h = HighlightLines::new(syntax, &self.theme);
            let mut highlighted = String::new();
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

    fn wrap_code(code: &str, width: usize) -> String {
        let mut result = String::new();
        for line in code.lines() {
            if line.len() <= width {
                result.push_str(line);
                result.push('\n');
            } else {
                let mut start = 0;
                while start < line.len() {
                    let end = (start + width).min(line.len());
                    result.push_str(&line[start..end]);
                    result.push('\n');
                    start = end;
                }
            }
        }
        result
    }
}

pub struct MarkdownWriter<W: io::Write> {
    buffer: String,
    renderer: MarkdownRenderer,
    previous_rendered: String,
    writer: W,
}

impl<W: io::Write> MarkdownWriter<W> {
    pub fn new(renderer: MarkdownRenderer, writer: W) -> Self {
        Self {
            buffer: String::new(),
            renderer,
            previous_rendered: String::new(),
            writer,
        }
    }

    pub fn add_chunk(&mut self, chunk: &str) -> io::Result<()> {
        self.buffer.push_str(chunk);

        if let Some(rendered) = self.try_render()? {
            self.stream(&rendered)?;
        }

        Ok(())
    }

    fn try_render(&mut self) -> io::Result<Option<String>> {
        let result = self.renderer.render(&self.buffer);
        Ok(Some(result))
    }

    pub fn flush(&mut self) -> io::Result<Option<String>> {
        if !self.buffer.is_empty() {
            let result = self.renderer.render(&self.buffer);
            self.buffer.clear();
            self.previous_rendered.clear();
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    pub fn stream(&mut self, content: &str) -> io::Result<()> {
        let rendered_lines: Vec<&str> = content.lines().collect();
        let lines_new: Vec<&str> = rendered_lines;
        let lines_prev: Vec<&str> = self.previous_rendered.lines().collect();
        let common = lines_prev
            .iter()
            .zip(&lines_new)
            .take_while(|(p, n)| p == n)
            .count();
        if common < lines_prev.len() {
            let up_lines = lines_prev.len() - common;
            if up_lines > 0 {
                self.writer
                    .write_all(format!("\x1b[{}A", up_lines).as_bytes())?;
            }
            self.writer.write_all(b"\x1b[0J")?;
        }
        for line in &lines_new[common..] {
            self.writer
                .write_all(format!("{}\x1b[K\n", line).as_bytes())?;
        }
        self.writer.flush()?;
        self.previous_rendered = content.to_string();
        Ok(())
    }
}
