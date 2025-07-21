use edtui::{EditorTheme, EditorView};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols::{border, line};
use ratatui::widgets::{Block, Borders, Padding, StatefulWidget, Widget};

use crate::domain::{MenuItem, State};
use crate::widgets::menu::MenuWidget;
use crate::widgets::message_list::MessageList;
use crate::widgets::spotlight::SpotlightWidget;
use crate::widgets::status_bar::StatusBar;
use crate::widgets::welcome::WelcomeWidget;

/// Chat widget that handles the chat interface with editor and message list
#[derive(Clone, Default)]
pub struct ChatWidget;

impl StatefulWidget for ChatWidget {
    type State = State;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut State,
    ) where
        Self: Sized,
    {
        // Create chat layout with messages area at top and user input area at bottom
        let chat_layout = Layout::new(
            Direction::Vertical,
            [Constraint::Fill(0), Constraint::Max(5)],
        );
        let [messages_area, user_area] = chat_layout.areas(area);

        // Messages area block (now at top)
        let message_block = Block::new();

        // Render welcome widget if no messages, otherwise render message list
        if state.messages.is_empty() {
            WelcomeWidget.render(message_block.inner(messages_area), buf, state);
        } else {
            MessageList.render(message_block.inner(messages_area), buf, state);
        }

        if state.spotlight.is_visible {
            // SpotlightWidget.render(messages_area, buf, state)

            MenuWidget::new(vec![
                MenuItem::new("Agent", "Switch between different agents", 'a'),
                MenuItem::new(
                    "Compact",
                    "Start new conversation with summarized context",
                    'c',
                ),
                MenuItem::new("Dump", "Export conversation as JSON or HTML", 'd'),
                MenuItem::new("Quit", "Close the application", 'q'),
                MenuItem::new("Forge", "Switch to agent Forge", 'f'),
                MenuItem::new("Help", "Access help documentation and instructions", 'h'),
                MenuItem::new("Info", "Display system and environment information", 'i'),
                MenuItem::new("Login", "Authenticate with Forge account", 'l'),
                MenuItem::new("Logout", "Sign out from current session", 'o'),
                MenuItem::new("Model", "Switch to different AI model", 'm'),
                MenuItem::new("Muse", "Switch to agent Muse", 'u'),
                MenuItem::new("New", "Start new conversation", 'n'),
                MenuItem::new("Tools", "View available tools", 't'),
                MenuItem::new("Update", "Upgrade to latest Forge version", 'p'),
            ])
            .render(messages_area, buf, state);
        }

        // User input area block with status bar (now at bottom)
        let user_block = Block::bordered()
            .padding(Padding::new(0, 0, 0, 1))
            .border_style(Style::default().dark_gray())
            .border_set(if state.spotlight.is_visible {
                border::Set {
                    top_left: line::VERTICAL_RIGHT,
                    top_right: line::VERTICAL_LEFT,
                    ..border::PLAIN
                }
            } else {
                border::PLAIN
            })
            .title_bottom(StatusBar::new(
                "FORGE",
                state.editor.mode.name(),
                state.workspace.clone(),
            ));

        EditorView::new(&mut state.editor)
            .theme(
                EditorTheme::default()
                    .base(Style::reset())
                    .cursor_style(Style::default().fg(Color::Black).bg(Color::White))
                    .hide_status_line(),
            )
            .wrap(true)
            .render(user_block.inner(user_area), buf);

        // Render blocks
        message_block.render(messages_area, buf);
        user_block.render(user_area, buf);
    }
}
