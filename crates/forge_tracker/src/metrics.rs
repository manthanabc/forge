
use forge_domain::metrics::{Metrics, SessionSummary};
use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Tracks metrics for individual file changes during a session
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileChangeMetrics {
    /// Total lines added to this file
    pub lines_added: u64,
    /// Total lines removed from this file
    pub lines_removed: u64,
    /// Number of operations performed on this file
    pub operations_count: u64,
}

impl FileChangeMetrics {
    /// Creates a new FileChangeMetrics instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds operation metrics to this file's tracking
    pub fn add_operation(&mut self, lines_added: u64, lines_removed: u64) {
        self.lines_added += lines_added;
        self.lines_removed += lines_removed;
        self.operations_count += 1;
    }

    /// Gets the net change in lines for this file
    pub fn net_change(&self) -> i64 {
        self.lines_added as i64 - self.lines_removed as i64
    }
}



/// Aggregates session metrics including file changes, operations, and duration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsService {
    /// When the session started tracking
    pub start_time: Option<DateTime<Utc>>,
    /// Tracks changes per file path
    pub files_changed: HashMap<String, FileChangeMetrics>,
    /// Total lines added across all files
    pub total_lines_added: u64,
    /// Total lines removed across all files
    pub total_lines_removed: u64,
    /// Total number of file operations performed
    pub operations_count: u64,
}

impl MetricsService {
    /// Creates a new MetricsService instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the session duration if tracking has started
    pub fn duration(&self) -> Option<Duration> {
        self.start_time.map(|start| (Utc::now() - start).to_std().unwrap_or_default())
    }

    /// Gets the number of unique files changed
    pub fn files_changed_count(&self) -> usize {
        self.files_changed.len()
    }

    /// Gets the net change in lines across all files
    pub fn net_change(&self) -> i64 {
        self.total_lines_added as i64 - self.total_lines_removed as i64
    }

    /// Resets all metrics to their initial state
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Metrics for MetricsService {
    fn start(&mut self) {
        self.start_time = Some(Utc::now());
    }

    fn record_file_operation(&mut self, path: String, lines_added: u64, lines_removed: u64) {
        // Update file-specific metrics
        let file_metrics = self.files_changed.entry(path).or_default();
        file_metrics.add_operation(lines_added, lines_removed);

        // Update totals
        self.total_lines_added += lines_added;
        self.total_lines_removed += lines_removed;
        self.operations_count += 1;
    }

    fn summary(&self) -> SessionSummary {
        let duration = match self.duration() {
            Some(d) => {
                let total_seconds = d.as_secs();
                let minutes = total_seconds / 60;
                let seconds = total_seconds % 60;
                format!("{}m {}s", minutes, seconds)
            }
            None => "0m 0s".to_string(),
        };

        SessionSummary {
            duration,
            files_changed: self.files_changed_count(),
            lines_added: self.total_lines_added,
            lines_removed: self.total_lines_removed,
            net_change: self.net_change(),
            operations: self.operations_count,
        }
    }
}
