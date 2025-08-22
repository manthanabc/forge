use super::operation::PermissionOperation;
use super::policy::Policy;
use crate::PolicyConfig;
use crate::policies::Permission;

/// High-level policy engine that provides convenient methods for checking
/// policies
///
/// This wrapper around Workflow provides easy-to-use methods for services to
/// check if operations are allowed without having to construct Operation enums
/// manually.
pub struct PolicyEngine<'a> {
    policies: &'a PolicyConfig,
}

impl<'a> PolicyEngine<'a> {
    /// Create a new PolicyEngine from a workflow
    pub fn new(policies: &'a PolicyConfig) -> Self {
        Self { policies }
    }

    /// Check if an operation is allowed
    /// Returns permission result
    pub fn can_perform(&self, operation: &PermissionOperation) -> Permission {
        self.evaluate_policies(operation)
    }

    /// Internal helper function to evaluate policies for a given operation
    /// Returns permission result, defaults to Confirm if no policies match
    fn evaluate_policies(&self, operation: &PermissionOperation) -> Permission {
        let has_policies = !self.policies.policies.is_empty();

        if !has_policies {
            return Permission::Confirm;
        }

        let mut last_allow: Option<Permission> = None;

        // Evaluate all policies in order: workflow policies first, then extended
        // policies

        if let Some(permission) = self.evaluate_policy_set(self.policies.policies.iter(), operation)
        {
            match permission {
                Permission::Deny | Permission::Confirm => {
                    // Return immediately for denials or confirmations
                    return permission;
                }
                Permission::Allow => {
                    // Keep track of the last allow
                    last_allow = Some(permission);
                }
            }
        }

        // Return last allow if found, otherwise default to Confirm
        last_allow.unwrap_or(Permission::Confirm)
    }

    /// Helper function to evaluate a set of policies
    /// Returns the first non-Allow result, or the last Allow result if all are
    /// Allow
    fn evaluate_policy_set<'p, I: IntoIterator<Item = &'p Policy>>(
        &self,
        policies: I,
        operation: &PermissionOperation,
    ) -> Option<Permission> {
        let mut last_allow: Option<Permission> = None;

        for policy in policies {
            if let Some(permission) = policy.eval(operation) {
                match permission {
                    Permission::Deny | Permission::Confirm => {
                        // Return immediately for denials or confirmations
                        return Some(permission);
                    }
                    Permission::Allow => {
                        // Keep track of the last allow
                        last_allow = Some(permission);
                    }
                }
            }
        }

        last_allow
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{ExecuteRule, Fetch, Permission, Policy, PolicyConfig, ReadRule, Rule, WriteRule};

    fn fixture_workflow_with_read_policy() -> PolicyConfig {
        let policies = PolicyConfig::new().add_policy(Policy::Simple {
            permission: Permission::Allow,
            rule: Rule::Read(ReadRule { read: "src/**/*.rs".to_string(), dir: None }),
        });
        policies
    }

    fn fixture_workflow_with_write_policy() -> PolicyConfig {
        let policies = PolicyConfig::new().add_policy(Policy::Simple {
            permission: Permission::Deny,
            rule: Rule::Write(WriteRule { write: "**/*.rs".to_string(), dir: None }),
        });
        policies
    }

    fn fixture_workflow_with_execute_policy() -> PolicyConfig {
        let policies = PolicyConfig::new().add_policy(Policy::Simple {
            permission: Permission::Allow,
            rule: Rule::Execute(ExecuteRule { command: "cargo *".to_string(), dir: None }),
        });
        policies
    }

    fn fixture_workflow_with_write_policy_confirm() -> PolicyConfig {
        let policies = PolicyConfig::new().add_policy(Policy::Simple {
            permission: Permission::Confirm,
            rule: Rule::Write(WriteRule { write: "src/**/*.rs".to_string(), dir: None }),
        });
        policies
    }

    fn fixture_workflow_with_net_fetch_policy() -> PolicyConfig {
        let policies = PolicyConfig::new().add_policy(Policy::Simple {
            permission: Permission::Allow,
            rule: Rule::Fetch(Fetch { url: "https://api.example.com/*".to_string(), dir: None }),
        });
        policies
    }

    #[test]
    fn test_policy_engine_can_perform_read() {
        let fixture_workflow = fixture_workflow_with_read_policy();
        let fixture = PolicyEngine::new(&fixture_workflow);
        let operation = PermissionOperation::Read {
            path: std::path::PathBuf::from("src/main.rs"),
            cwd: std::path::PathBuf::from("/test/cwd"),
            message: "Read file: src/main.rs".to_string(),
        };

        let actual = fixture.can_perform(&operation);

        assert_eq!(actual, Permission::Allow);
    }

    #[test]
    fn test_policy_engine_can_perform_write() {
        let fixture_workflow = fixture_workflow_with_write_policy();
        let fixture = PolicyEngine::new(&fixture_workflow);
        let operation = PermissionOperation::Write {
            path: std::path::PathBuf::from("src/main.rs"),
            cwd: std::path::PathBuf::from("/test/cwd"),
            message: "Create/overwrite file: src/main.rs".to_string(),
        };

        let actual = fixture.can_perform(&operation);

        assert_eq!(actual, Permission::Deny);
    }

    #[test]
    fn test_policy_engine_can_perform_write_with_confirm() {
        let fixture_workflow = fixture_workflow_with_write_policy_confirm();
        let fixture = PolicyEngine::new(&fixture_workflow);
        let operation = PermissionOperation::Write {
            path: std::path::PathBuf::from("src/main.rs"),
            cwd: std::path::PathBuf::from("/test/cwd"),
            message: "Create/overwrite file: src/main.rs".to_string(),
        };

        let actual = fixture.can_perform(&operation);

        assert_eq!(actual, Permission::Confirm);
    }

    #[test]
    fn test_policy_engine_can_perform_execute() {
        let fixture_workflow = fixture_workflow_with_execute_policy();
        let fixture = PolicyEngine::new(&fixture_workflow);
        let operation = PermissionOperation::Execute {
            command: "cargo build".to_string(),
            cwd: std::path::PathBuf::from("/test/cwd"),
            message: "Execute shell command: cargo build".to_string(),
        };

        let actual = fixture.can_perform(&operation);

        assert_eq!(actual, Permission::Allow);
    }

    #[test]
    fn test_policy_engine_can_perform_net_fetch() {
        let fixture_workflow = fixture_workflow_with_net_fetch_policy();
        let fixture = PolicyEngine::new(&fixture_workflow);
        let operation = PermissionOperation::Fetch {
            url: "https://api.example.com/data".to_string(),
            cwd: std::path::PathBuf::from("/test/cwd"),
            message: "Fetch content from URL: https://api.example.com/data".to_string(),
        };

        let actual = fixture.can_perform(&operation);

        assert_eq!(actual, Permission::Allow);
    }
}
