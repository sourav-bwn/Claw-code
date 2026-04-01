use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub permission_level: PermissionLevel,
    pub allowed_directories: Vec<PathBuf>,
    pub blocked_commands: Vec<String>,
    pub max_bash_timeout: u64,
    pub enable_hooks: bool,
    pub hooks_config: HooksConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionLevel {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            permission_level: PermissionLevel::DangerFullAccess,
            allowed_directories: Vec::new(),
            blocked_commands: vec![
                "rm -rf /".to_string(),
                "sudo".to_string(),
                "dd".to_string(),
            ],
            max_bash_timeout: 120,
            enable_hooks: true,
            hooks_config: HooksConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksConfig {
    pub pre_tool_use: Vec<HookEntry>,
    pub post_tool_use: Vec<HookEntry>,
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            pre_tool_use: Vec::new(),
            post_tool_use: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEntry {
    pub tool: String,
    pub command: String,
}

impl RuntimeConfig {
    pub fn load() -> Self {
        Self::load_from_default_locations()
    }

    pub fn load_from_default_locations() -> Self {
        let config_paths = [
            PathBuf::from(".uc law.json"),
            PathBuf::from(".uc law/config.json"),
            dirs::config_dir()
                .map(|p| p.join("uc law").join("config.json"))
                .unwrap_or_default(),
        ];

        for path in &config_paths {
            if path.exists() {
                if let Ok(config) = Self::load_from_file(path) {
                    return config;
                }
            }
        }

        Self::default()
    }

    pub fn load_from_file(path: impl AsRef<std::path::Path>) -> Result<Self, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn save_to_file(&self, path: impl AsRef<std::path::Path>) -> Result<(), std::io::Error> {
        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, contents)
    }
}

use super::permissions::PermissionLevel;
