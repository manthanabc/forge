#[derive(Clone, Debug)]
pub struct MenuItem {
    pub title: String,
    pub description: String,
    pub trigger_letter: char,
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
            trigger_letter,
        }
    }
}
