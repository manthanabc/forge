use std::collections::HashMap;

use derive_setters::Setters;
use forge_domain::{Compact, MaxTokens, ModelId, Provider, Temperature, TopK, TopP, Update};
use serde_json::Value;

#[derive(Debug, Clone, Setters)]
#[setters(strip_option, into)]
pub struct Profile {
    pub name: String,
    pub provider: Provider,
    pub is_active: bool,

    // Fields from Workflow (excluding agents)
    /// Path pattern for custom template files (supports glob patterns)
    pub templates: Option<String>,

    /// Variables that can be used in templates
    pub variables: HashMap<String, Value>,

    /// configurations that can be used to update forge
    pub updates: Option<Update>,

    /// Default model ID to use for agents in this workflow
    pub model: Option<ModelId>,

    /// Maximum depth to which the file walker should traverse for all agents
    /// If not provided, each agent's individual setting will be used
    pub max_walker_depth: Option<usize>,

    /// A set of custom rules that all agents should follow
    /// These rules will be applied in addition to each agent's individual rules
    pub custom_rules: Option<String>,

    /// Temperature used for all agents
    ///
    /// Temperature controls the randomness in the model's output.
    /// - Lower values (e.g., 0.1) make responses more focused, deterministic,
    ///   and coherent
    /// - Higher values (e.g., 0.8) make responses more creative, diverse, and
    ///   exploratory
    /// - Valid range is 0.0 to 2.0
    /// - If not specified, each agent's individual setting or the model
    ///   provider's default will be used
    pub temperature: Option<Temperature>,

    /// Top-p (nucleus sampling) used for all agents
    ///
    /// Controls the diversity of the model's output by considering only the
    /// most probable tokens up to a cumulative probability threshold.
    /// - Lower values (e.g., 0.1) make responses more focused
    /// - Higher values (e.g., 0.9) make responses more diverse
    /// - Valid range is 0.0 to 1.0
    /// - If not specified, each agent's individual setting or the model
    ///   provider's default will be used
    pub top_p: Option<TopP>,

    /// Top-k used for all agents
    ///
    /// Controls the number of highest probability vocabulary tokens to keep.
    /// - Lower values (e.g., 10) make responses more focused
    /// - Higher values (e.g., 100) make responses more diverse
    /// - Valid range is 1 to 1000
    /// - If not specified, each agent's individual setting or the model
    ///   provider's default will be used
    pub top_k: Option<TopK>,

    /// Maximum number of tokens the model can generate for all agents
    ///
    /// Controls the maximum length of the model's response.
    /// - Lower values (e.g., 100) limit response length for concise outputs
    /// - Higher values (e.g., 4000) allow for longer, more detailed responses
    /// - Valid range is 1 to 100,000
    /// - If not specified, each agent's individual setting or the model
    ///   provider's default will be used
    pub max_tokens: Option<MaxTokens>,

    /// Flag to enable/disable tool support for all agents in this workflow.
    /// If not specified, each agent's individual setting will be used.
    /// Default is false (tools disabled) when not specified.
    pub tool_supported: Option<bool>,

    /// Maximum number of times a tool can fail before the orchestrator
    /// forces the completion.
    pub max_tool_failure_per_turn: Option<usize>,

    /// Maximum number of requests that can be made in a single turn
    pub max_requests_per_turn: Option<usize>,

    /// Configuration for automatic context compaction for all agents
    /// If specified, this will be applied to all agents in the workflow
    /// If not specified, each agent's individual setting will be used
    pub compact: Option<Compact>,
}
