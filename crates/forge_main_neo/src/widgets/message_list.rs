use ansi_to_tui::IntoText;
use forge_api::ChatResponse;
use ratatui::layout::Size;
use ratatui::widgets::Padding;
use ratatui::prelude::{Buffer, Rect, Widget};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, StatefulWidget, Wrap};
use tui_scrollview::ScrollView;

use crate::domain::{Message, State};
use crate::widgets::spinner::Spinner;

#[derive(Default)]
pub struct MessageList;

fn single_message_to_lines(message: &Message) -> Vec<Line<'_>> {
    match message {
        Message::User(content) => vec![Line::from(vec![
            Span::styled("❯ ", Style::default().green()),
            Span::styled(content, Style::default().cyan().bold()),
        ])],
        Message::Assistant(response) => match response {
            ChatResponse::Text {
                text,
                is_complete,
                is_md,
            } => {
                if *is_complete {
                    if *is_md {
                        let rendered_text = forge_display::MarkdownFormat::new().render(text);
                        match rendered_text.into_text() {
                            Ok(text) => text.lines,
                            Err(_) => vec![Line::raw(rendered_text)],
                        }
                    } else {
                        match text.clone().into_text() {
                            Ok(text) => text.lines,
                            Err(_) => vec![Line::raw(text.clone())],
                        }
                    }
                } else {
                    vec![]
                }
            }
            ChatResponse::ToolCallStart(_) => vec![],
            ChatResponse::ToolCallEnd(_) => vec![],
            ChatResponse::Usage(_) => vec![],
            ChatResponse::Interrupt { reason: _ } => {
                todo!()
            }
            ChatResponse::Reasoning { content } => {
                if !content.trim().is_empty() {
                    vec![Line::from(vec![
                        Span::styled("Thinking... ", Style::default().dark_gray()),
                        Span::styled(format!("{}", content), Style::default().dim()),
                    ])]
                } else {
                    vec![]
                }
            }
            ChatResponse::Summary { content } => {
                if !content.trim().is_empty() {
                    let rendered_text = forge_display::MarkdownFormat::new().render(content);
                    match rendered_text.into_text() {
                        Ok(text) => {
                            let mut lines = vec![Line::from(Span::styled(
                                "Summary:",
                                Style::default().dark_gray(),
                            ))];
                            lines.extend(text.lines);
                            lines
                        }
                        Err(_) => vec![],
                    }
                } else {
                    vec![]
                }
            }
            ChatResponse::RetryAttempt {
                cause: _,
                duration: _,
            } => {
                todo!()
            }
        },
    }
}

fn get_paragraph_height(lines: &[Line<'_>], width: u16) -> u16 {
    if width == 0 {
        return lines.len() as u16;
    }
    lines
        .iter()
        .map(|line| {
            let line_width = line.width() as u16;
            if line_width == 0 {
                1
            } else {
                (line_width + width - 1) / width
            }
        })
        .sum()
}

fn calculate_total_height(state: &State, width: u16) -> u16 {
    let mut total_height = 0;
    for message in &state.messages {
        let lines = single_message_to_lines(message);
        if lines.is_empty() {
            continue;
        }
        let para_height = get_paragraph_height(&lines, width);
        total_height += para_height + 2 + 1; // +2 for borders, +1 for margin
    }
    if state.show_spinner {
        total_height += 1;
    }
    total_height
}

struct MessagesRenderer<'a> {
    state: &'a mut State,
}

impl<'a> Widget for MessagesRenderer<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut y = 0;
        for message in &self.state.messages {
            let lines = single_message_to_lines(message);
            if lines.is_empty() {
                continue;
            }

            let para_height = get_paragraph_height(&lines, area.width - 4);
            if para_height == 0 {
                continue;
            }
            let block_height = para_height + 2; // for borders

            if y + block_height > area.height {
                break;
            }

            let message_area = Rect::new(area.x, area.y + y, area.width, block_height);

            let title = match message {
                Message::User(_) => "User",
                Message::Assistant(_) => "Assistant",
            };
            let block = Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::DarkGray))
                .border_type(ratatui::widgets::BorderType::Rounded);
                .padding(Padding::new(1, 0, 0, 1));
            let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });

            paragraph.block(block).render(message_area, buf);

            y += block_height + 1; // +1 for margin
        }

        // if self.state.show_spinner {
        //     let spinner = Spinner::default();
        //     let spinner_line = spinner.to_line(self.state);
        //     let spinner_height = 1;
        //     if y + spinner_height <= area.height {
        //         let spinner_area = Rect::new(area.x, area.y + y, area.width, spinner_height);
        //         Paragraph::new(spinner_line).render(spinner_area, buf);
        //     }
        // }
    }
}

impl StatefulWidget for MessageList {
    type State = State;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut State)
    where
        Self: Sized,
    {
        let width = area.width;
        let total_height = calculate_total_height(state, width - 2);

        let mut scroll_view = ScrollView::new(Size::new(width, total_height))
            .horizontal_scrollbar_visibility(tui_scrollview::ScrollbarVisibility::Never);

        let renderer = MessagesRenderer { state };
        scroll_view.render_widget(renderer, scroll_view.area());

        scroll_view.render(area, buf, &mut state.message_scroll_state);
    }
}
