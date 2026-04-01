use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionLevel {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

impl Default for PermissionLevel {
    fn default() -> Self {
        PermissionLevel::DangerFullAccess
    }
}

impl std::fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionLevel::ReadOnly => write!(f, "read-only"),
            PermissionLevel::WorkspaceWrite => write!(f, "workspace-write"),
            PermissionLevel::DangerFullAccess => write!(f, "danger-full-access"),
        }
    }
}

pub struct PermissionManager {
    level: PermissionLevel,
    allowed_paths: Vec<PathBuf>,
    blocked_commands: Vec<String>,
}

impl PermissionManager {
    pub fn new(level: PermissionLevel) -> Self {
        Self {
            level,
            allowed_paths: Vec::new(),
            blocked_commands: Vec::new(),
        }
    }

    pub fn with_allowed_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.allowed_paths = paths;
        self
    }

    pub fn with_blocked_commands(mut self, commands: Vec<String>) -> Self {
        self.blocked_commands = commands;
        self
    }

    pub fn level(&self) -> PermissionLevel {
        self.level
    }

    pub fn set_level(&mut self, level: PermissionLevel) {
        self.level = level;
    }

    pub fn can_read_path(&self, path: &Path) -> bool {
        match self.level {
            PermissionLevel::ReadOnly => true,
            PermissionLevel::WorkspaceWrite => self.is_path_allowed(path),
            PermissionLevel::DangerFullAccess => true,
        }
    }

    pub fn can_write_path(&self, path: &Path) -> bool {
        match self.level {
            PermissionLevel::ReadOnly => false,
            PermissionLevel::WorkspaceWrite => self.is_path_allowed(path),
            PermissionLevel::DangerFullAccess => true,
        }
    }

    pub fn can_execute_command(&self, command: &str) -> bool {
        if self.level == PermissionLevel::ReadOnly {
            return false;
        }

        for blocked in &self.blocked_commands {
            if command.contains(blocked) {
                return false;
            }
        }

        true
    }

    pub fn can_run_bash(&self) -> bool {
        self.level != PermissionLevel::ReadOnly
    }

    fn is_path_allowed(&self, path: &Path) -> bool {
        if self.allowed_paths.is_empty() {
            return true;
        }

        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        
        for allowed in &self.allowed_paths {
            let allowed_canonical = allowed.canonicalize().unwrap_or_else(|_| allowed.clone());
            if canonical.starts_with(&allowed_canonical) {
                return true;
            }
        }

        false
    }

    pub fn add_allowed_path(&mut self, path: PathBuf) {
        self.allowed_paths.push(path);
    }

    pub fn add_blocked_command(&mut self, command: String) {
        self.blocked_commands.push(command);
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new(PermissionLevel::default())
    }
}
