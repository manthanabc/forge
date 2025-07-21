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
            MenuItem::new(
                "Agent",
                "Switch between different AI agents for your requests",
                'a',
            ),
            MenuItem::new(
                "Compact",
                "Create a condensed version of the current conversation",
                'c',
            ),
            MenuItem::new("Dump", "Export conversation to JSON or HTML format", 'd'),
            MenuItem::new("Quit", "Close the application and end the session", 'q'),
            MenuItem::new(
                "Forge",
                "Enable implementation mode for making code changes",
                'f',
            ),
            MenuItem::new(
                "Help",
                "Access help documentation and usage instructions",
                'h',
            ),
            MenuItem::new("Info", "Display system and environment information", 'i'),
            MenuItem::new(
                "Login",
                "Authenticate with your Forge provider account",
                'l',
            ),
            MenuItem::new("Logout", "Sign out from your current session", 'o'),
            MenuItem::new("Model", "Switch to a different AI model", 'm'),
            MenuItem::new(
                "Muse",
                "Enable planning mode without executing code changes",
                'u',
            ),
            MenuItem::new("New", "Start a fresh conversation session", 'n'),
            MenuItem::new("Tools", "View all available tools with descriptions", 't'),
            MenuItem::new("Update", "Upgrade to the latest version of Forge", 'p'),
        ])
        .render(area, buf, state);
    }
}
