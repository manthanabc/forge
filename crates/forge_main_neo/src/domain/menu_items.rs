use crate::domain::MenuItem;

/// Central definition and management of all menu items
#[derive(Debug, Clone)]
pub struct MenuItems {
    items: Vec<MenuItem>,
}

impl Default for MenuItems {
    fn default() -> Self {
        Self::new()
    }
}

impl MenuItems {
    pub fn new() -> Self {
        Self {
            items: vec![
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
            ],
        }
    }

    pub fn count(&self) -> usize {
        self.items.len()
    }

    pub fn to_vec(&self) -> Vec<MenuItem> {
        self.items.clone()
    }
}
