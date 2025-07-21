use ratatui::widgets::StatefulWidget;

use crate::domain::{MenuItem, State};
use crate::widgets::chat::ChatWidget;
use crate::widgets::menu::Menu;

#[derive(Clone, Default)]
pub struct App;

impl StatefulWidget for App {
    type State = State;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut State,
    ) where
        Self: Sized,
    {
        ChatWidget.render(area, buf, state);
        Menu::new(vec![
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
        .render(area, buf, state);
    }
}
