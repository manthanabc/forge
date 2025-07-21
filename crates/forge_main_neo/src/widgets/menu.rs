use ratatui::layout::*;
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols::{border, line};
use ratatui::text::{Line, Span};
use ratatui::widgets::*;

use crate::domain::{MenuItem, State};

pub struct Menu {
    items: Vec<MenuItem>,
}

impl Menu {
    pub fn new(items: Vec<MenuItem>) -> Self {
        Self { items }
    }

    fn render_menu_item(item: MenuItem, max_title_width: usize) -> Line<'static> {
        let max_title_width = max_title_width + 1;
        let char = item.trigger_letter;
        let title = format!("{:<max_title_width$}", item.title);
        let description = item.description;

        let mut spans: Vec<Span<'static>> = Vec::new();
        spans.push(Span::styled(format!("[{char}] "), Style::new().dim()));

        // Find the first occurrence of the trigger letter in the title (case
        // insensitive)
        if let Some(pos) = title.to_lowercase().as_str().find(char.to_ascii_lowercase()) {
            // Add text before the trigger letter
            if pos > 0 {
                spans.push(Span::styled(
                    title[..pos].to_owned(),
                    Style::default().cyan().bold(),
                ));
            }

            // Add the highlighted trigger letter
            spans.push(Span::styled(
                title[pos..pos + 1].to_owned(),
                Style::default().bold().cyan().underlined(),
            ));

            // Add text after the trigger letter
            if pos + 1 < title.len() {
                spans.push(Span::styled(
                    title[pos + 1..].to_owned(),
                    Style::default().cyan().bold(),
                ));
            }
        } else {
            // Fallback if trigger letter not found in title
            spans.push(Span::styled(title, Style::default().cyan().bold()))
        }

        // Add description with calculated padding
        spans.push(Span::styled(description.to_string(), Style::new().green()));

        Line::from(spans)
    }
}

impl StatefulWidget for Menu {
    type State = State;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        _state: &mut Self::State,
    ) {
        let [area] = Layout::vertical([Constraint::Percentage(75)])
            .flex(Flex::Center)
            .areas(area);

        let [area] = Layout::horizontal([Constraint::Percentage(50)])
            .flex(Flex::Center)
            .areas(area);

        Clear.render(area, buf);

        let menu_block = Block::bordered()
            .border_set(border::Set {
                bottom_right: line::VERTICAL_LEFT,
                bottom_left: line::VERTICAL_RIGHT,
                ..border::PLAIN
            })
            .title(" MENU ")
            .title_style(Style::default().bold())
            .border_style(Style::default().fg(Color::Blue));

        // Calculate the maximum title width for consistent description alignment
        let max_title_width = self
            .items
            .iter()
            .map(|item| item.title.len())
            .max()
            .unwrap_or(0);

        let items = self
            .items
            .into_iter()
            .map(|item| Self::render_menu_item(item, max_title_width))
            .collect::<Vec<_>>();

        let [menu_area, description_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Max(3)])
                .flex(Flex::SpaceAround)
                .areas(area);

        Paragraph::new(items)
            .block(menu_block)
            .render(menu_area, buf);

        let description_block = Block::bordered()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(Color::Blue));

        // TODO: use description of the selected item
        Paragraph::new(vec![
            Line::from("Shortcut: `N`").style(Style::new().dim()),
            Line::from("Creates a compact version of the conversation").style(Style::new().dim()),
        ])
        .block(description_block)
        .render(description_area, buf);
    }
}
