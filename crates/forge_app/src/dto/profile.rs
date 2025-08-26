#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub provider: String,
    pub is_active: bool,
    pub model_name: Option<String>,
}
