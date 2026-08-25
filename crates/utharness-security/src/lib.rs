use std::path::{Path, PathBuf};
use utharness_core::{PermissionDecision, PermissionMode, ToolRequest};

#[derive(Clone, Debug)]
pub struct Policy {
    pub mode: PermissionMode,
    pub workspace: PathBuf,
    pub allow_network: bool,
    pub allow_shell: bool,
}

impl Policy {
    pub fn safe(workspace: impl Into<PathBuf>) -> Self {
        Self {
            mode: PermissionMode::Safe,
            workspace: workspace.into(),
            allow_network: false,
            allow_shell: false,
        }
    }

    pub fn evaluate(&self, request: &ToolRequest) -> PermissionDecision {
        let readonly = matches!(
            request.tool.as_str(),
            "read_file"
                | "list_directory"
                | "search_files"
                | "glob"
                | "file_info"
                | "git_status"
                | "git_diff"
                | "git_log"
                | "code_search"
        );
        match self.mode {
            PermissionMode::Safe => {
                if readonly {
                    PermissionDecision::Allow
                } else {
                    PermissionDecision::Deny
                }
            }
            PermissionMode::Ask => {
                if readonly {
                    PermissionDecision::Allow
                } else {
                    PermissionDecision::Prompt
                }
            }
            PermissionMode::Trusted => PermissionDecision::Allow,
            PermissionMode::Custom => {
                if readonly {
                    PermissionDecision::Allow
                } else {
                    PermissionDecision::Prompt
                }
            }
        }
    }

    pub fn validate_path(&self, requested: &Path) -> Result<PathBuf, String> {
        let candidate = if requested.is_absolute() {
            requested.to_path_buf()
        } else {
            self.workspace.join(requested)
        };
        let normalized = candidate
            .canonicalize()
            .map_err(|e| format!("cannot resolve path: {e}"))?;
        let root = self
            .workspace
            .canonicalize()
            .map_err(|e| format!("cannot resolve workspace: {e}"))?;
        if normalized == root || normalized.starts_with(&root) {
            Ok(normalized)
        } else {
            Err(format!("path escapes workspace: {}", requested.display()))
        }
    }

    pub fn redact(text: &str) -> String {
        let mut output = text.to_string();
        for prefix in [
            "OPENAI_API_KEY=",
            "ANTHROPIC_API_KEY=",
            "API_KEY=",
            "TOKEN=",
            "SECRET=",
        ] {
            let mut search_from = 0;
            while let Some(relative_start) = output[search_from..].find(prefix) {
                let start = search_from + relative_start;
                let end = output[start + prefix.len()..]
                    .find([' ', '\n', '\r'])
                    .map(|i| start + prefix.len() + i)
                    .unwrap_or(output.len());
                output.replace_range(start + prefix.len()..end, "[REDACTED]");
                search_from = start + prefix.len() + "[REDACTED]".len();
                if search_from >= output.len() {
                    break;
                }
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn safe_policy_allows_only_read_tools() {
        let policy = Policy::safe(".");
        assert_eq!(
            policy.evaluate(&ToolRequest {
                tool: "read_file".into(),
                target: None,
                arguments: json!({})
            }),
            PermissionDecision::Allow
        );
        assert_eq!(
            policy.evaluate(&ToolRequest {
                tool: "shell".into(),
                target: None,
                arguments: json!({})
            }),
            PermissionDecision::Deny
        );
    }

    #[test]
    fn redacts_common_secret_assignments() {
        assert_eq!(Policy::redact("API_KEY=abc123 ok"), "API_KEY=[REDACTED] ok");
    }
}
