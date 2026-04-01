use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    pub name: String,
    pub tool: Option<String>,
    pub command: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    pub success: bool,
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_continuation: Option<bool>,
}

impl HookResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
            allow_continuation: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(message.into()),
            allow_continuation: Some(false),
        }
    }

    pub fn block() -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some("Tool execution blocked by hook".to_string()),
            allow_continuation: Some(false),
        }
    }

    pub fn allow() -> Self {
        Self {
            success: true,
            output: String::new(),
            error: None,
            allow_continuation: Some(true),
        }
    }
}

pub struct HookExecutor {
    hooks: Vec<Hook>,
}

impl HookExecutor {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn add_hook(&mut self, hook: Hook) {
        self.hooks.push(hook);
    }

    pub fn remove_hook(&mut self, name: &str) {
        self.hooks.retain(|h| h.name != name);
    }

    pub async fn execute_pre_tool(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
    ) -> HookResult {
        for hook in &self.hooks {
            if hook.enabled && hook.tool.as_deref() == Some(tool_name) {
                let result = self.run_hook(&hook.command, tool_name, input).await;
                if !result.allow_continuation.unwrap_or(true) {
                    return result;
                }
            }
        }
        HookResult::allow()
    }

    pub async fn execute_post_tool(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
        result: &str,
    ) -> HookResult {
        for hook in &self.hooks {
            if hook.enabled && hook.tool.as_deref() == Some(tool_name) {
                let hook_result = self.run_hook_with_result(&hook.command, tool_name, input, result).await;
                if !hook_result.allow_continuation.unwrap_or(true) {
                    return hook_result;
                }
            }
        }
        HookResult::allow()
    }

    async fn run_hook(
        &self,
        command: &str,
        tool_name: &str,
        input: &serde_json::Value,
    ) -> HookResult {
        let output = tokio::process::Command::new("bash")
            .args(["-c", command])
            .env("UCLAW_TOOL_NAME", tool_name)
            .env("UCLAW_TOOL_INPUT", input.to_string())
            .output()
            .await;

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();

                HookResult {
                    success: out.status.success(),
                    output: stdout,
                    error: if stderr.is_empty() { None } else { Some(stderr) },
                    allow_continuation: Some(out.status.success()),
                }
            }
            Err(e) => HookResult::error(format!("Hook execution failed: {}", e)),
        }
    }

    async fn run_hook_with_result(
        &self,
        command: &str,
        tool_name: &str,
        input: &serde_json::Value,
        tool_result: &str,
    ) -> HookResult {
        let output = tokio::process::Command::new("bash")
            .args(["-c", command])
            .env("UCLAW_TOOL_NAME", tool_name)
            .env("UCLAW_TOOL_INPUT", input.to_string())
            .env("UCLAW_TOOL_RESULT", tool_result)
            .output()
            .await;

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();

                HookResult {
                    success: out.status.success(),
                    output: stdout,
                    error: if stderr.is_empty() { None } else { Some(stderr) },
                    allow_continuation: Some(out.status.success()),
                }
            }
            Err(e) => HookResult::error(format!("Hook execution failed: {}", e)),
        }
    }
}

impl Default for HookExecutor {
    fn default() -> Self {
        Self::new()
    }
}
