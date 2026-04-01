use std::path::PathBuf;
use tokio::process::Command;
use std::time::Duration;

pub struct Sandbox {
    working_directory: Option<PathBuf>,
    allowed_paths: Vec<PathBuf>,
    blocked_commands: Vec<String>,
    timeout: Duration,
    env_vars: Vec<(String, String)>,
}

impl Sandbox {
    pub fn new() -> Self {
        Self {
            working_directory: None,
            allowed_paths: Vec::new(),
            blocked_commands: Vec::new(),
            timeout: Duration::from_secs(60),
            env_vars: Vec::new(),
        }
    }

    pub fn working_directory(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    pub fn allow_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.allowed_paths.push(path.into());
        self
    }

    pub fn block_command(mut self, command: impl Into<String>) -> Self {
        self.blocked_commands.push(command.into());
        self
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.push((key.into(), value.into()));
        self
    }

    pub fn is_command_allowed(&self, command: &str) -> bool {
        for blocked in &self.blocked_commands {
            if command.contains(blocked) {
                return false;
            }
        }
        true
    }

    pub async fn execute(&self, command: &str) -> Result<SandboxResult, SandboxError> {
        if !self.is_command_allowed(command) {
            return Err(SandboxError::BlockedCommand(command.to_string()));
        }

        let mut cmd = Command::new("bash");
        cmd.args(["-c", command]);

        if let Some(ref dir) = self.working_directory {
            cmd.current_dir(dir);
        }

        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        cmd.env("UCLAW_SANDBOX", "1");

        match tokio::time::timeout(self.timeout, cmd.output()).await {
            Ok(Ok(output)) => {
                Ok(SandboxResult {
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    exit_code: output.status.code(),
                    success: output.status.success(),
                })
            }
            Ok(Err(e)) => Err(SandboxError::ExecutionError(e.to_string())),
            Err(_) => Err(SandboxError::Timeout(self.timeout)),
        }
    }
}

impl Default for Sandbox {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct SandboxResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub success: bool,
}

#[derive(Debug)]
pub enum SandboxError {
    BlockedCommand(String),
    ExecutionError(String),
    Timeout(Duration),
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxError::BlockedCommand(cmd) => write!(f, "Command blocked: {}", cmd),
            SandboxError::ExecutionError(e) => write!(f, "Execution error: {}", e),
            SandboxError::Timeout(d) => write!(f, "Command timed out after {:?}", d),
        }
    }
}

impl std::error::Error for SandboxError {}
