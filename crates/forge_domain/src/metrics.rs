use serde::{Deserialize, Serialize};

/// Formatted session summary for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub duration: String,
    pub files_changed: usize,
    pub lines_added: u64,
    pub lines_removed: u64,
    pub net_change: i64,
    pub operations: u64,
}

pub trait Metrics: Send + Sync {
    fn start(&mut self);
    fn record_file_operation(&mut self, path: String, lines_added: u64, lines_removed: u64);
    fn summary(&self) -> SessionSummary;
}

impl std::fmt::Display for SessionSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SESSION SUMMARY")?;
        writeln!(f, "Duration: {}", self.duration)?;
        writeln!(f, "Files Changed: {}", self.files_changed)?;
        writeln!(f, "Lines Added: {}", self.lines_added)?;
        writeln!(f, "Lines Removed: {}", self.lines_removed)?;
        
        let net_change_sign = if self.net_change >= 0 { "+" } else { "" };
        writeln!(f, "Net Change: {}{} lines", net_change_sign, self.net_change)?;
        writeln!(f, "Operations: {}", self.operations)?;
        
        Ok(())
    }
}