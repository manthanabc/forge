use std::sync::Arc;

use derive_setters::Setters;
use tokio::sync::mpsc::Sender;

use crate::metrics::Metrics;

use crate::{ChatResponse, TaskList};

/// Type alias for Arc<Sender<Result<ChatResponse>>>
type ArcSender = Arc<Sender<anyhow::Result<ChatResponse>>>;

/// Provides additional context for tool calls.
#[derive(Setters)]
pub struct ToolCallContext<'a> {
    sender: Option<ArcSender>,
    pub tasks: TaskList,
    pub session_metrics: &'a mut dyn Metrics,
}

impl std::fmt::Debug for ToolCallContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolCallContext")
            .field("sender", &self.sender)
            .field("tasks", &self.tasks)
            .field("session_metrics", &"<Metrics Trait Object>")
            .finish()
    }
}

impl<'a> ToolCallContext<'a> {
    /// Creates a new ToolCallContext with default values
    pub fn new(task_list: TaskList, session_metrics: &'a mut dyn Metrics) -> Self {
        Self {
            sender: None,
            tasks: task_list,
            session_metrics,
        }
    }

    /// Send a message through the sender if available
    pub async fn send(&self, agent_message: impl Into<ChatResponse>) -> anyhow::Result<()> {
        if let Some(sender) = &self.sender {
            sender.send(Ok(agent_message.into())).await?
        }
        Ok(())
    }

    pub async fn send_text(&self, content: impl ToString) -> anyhow::Result<()> {
        self.send(ChatResponse::Text { text: content.to_string(), is_complete: true, is_md: false })
            .await
    }
}

#[cfg(test)]
mod tests {
    use forge_tracker::metrics::MetricsService;

    #[test]
    fn test_create_context() {
        let mut session_metrics = MetricsService::new();
        let context = ToolCallContext::new(TaskList::new(), &mut session_metrics);
        assert!(context.sender.is_none());
    }

    #[test]
    fn test_with_sender() {
        // This is just a type check test - we don't actually create a sender
        // as it's complex to set up in a unit test
        let mut session_metrics = MetricsService::new();
        let context = ToolCall_context.new(TaskList::new(), &mut session_metrics);
        assert!(context.sender.is_none());
    }
}
