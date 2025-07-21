use ratatui::widgets::ListState;

#[derive(Default, Clone, Debug)]
pub struct MenuState {
    pub list: ListState,
}

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub title: String,
    pub description: String,
    pub shortcut: char,
}

impl MenuItem {
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        trigger_letter: char,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            shortcut: trigger_letter,
        }
    }
}
