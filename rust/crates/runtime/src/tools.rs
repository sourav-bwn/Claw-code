use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub category: ToolCategory,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    FileSystem,
    Search,
    Web,
    Git,
    Terminal,
    Code,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl ToolResult {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: content.into(),
            error: None,
            metadata: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            content: String::new(),
            error: Some(message.into()),
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.get_or_insert_with(HashMap::new).insert(key.into(), value);
        self
    }
}

#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn spec(&self) -> ToolSpec;
    async fn execute(&self, input: serde_json::Value) -> ToolResult;
}

pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, ToolSpec>>>,
    handlers: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, tool: Arc<dyn Tool>) {
        let spec = tool.spec();
        self.tools.write().await.insert(spec.name.clone(), spec);
        self.handlers.write().await.insert(tool.name().to_string(), tool);
    }

    pub async fn unregister(&self, name: &str) -> bool {
        self.tools.write().await.remove(name);
        self.handlers.write().await.remove(name).is_some()
    }

    pub async fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.handlers.read().await.get(name).cloned()
    }

    pub async fn get_spec(&self, name: &str) -> Option<ToolSpec> {
        self.tools.read().await.get(name).cloned()
    }

    pub async fn list_tools(&self) -> Vec<ToolSpec> {
        self.tools.read().await.values().cloned().collect()
    }

    pub async fn list_by_category(&self, category: ToolCategory) -> Vec<ToolSpec> {
        self.tools.read().await
            .values()
            .filter(|t| t.category == category)
            .cloned()
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BashTool;

#[async_trait::async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute shell commands in a sandboxed environment"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "bash".to_string(),
            description: "Execute shell commands. Returns command output.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    },
                    "timeout": {
                        "type": "number",
                        "description": "Timeout in seconds (default: 60)"
                    },
                    "workingDirectory": {
                        "type": "string",
                        "description": "Working directory for the command"
                    }
                },
                "required": ["command"]
            }),
            category: ToolCategory::Terminal,
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let command = match input.get("command").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ToolResult::error("Missing 'command' parameter"),
        };

        let timeout = input.get("timeout").and_then(|v| v.as_u64()).unwrap_or(60);
        let working_dir = input.get("workingDirectory").and_then(|v| v.as_str());

        let output = tokio::process::Command::new("bash")
            .args(["-c", command])
            .current_dir(working_dir.unwrap_or("."))
            .output()
            .await;

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);

                if out.status.success() {
                    ToolResult::success(stdout.to_string())
                        .with_metadata("exit_code", serde_json::json!(0))
                        .with_metadata("stderr", serde_json::json!(stderr.to_string()))
                } else {
                    ToolResult::success(format!("stdout:\n{}\nstderr:\n{}", stdout, stderr))
                        .with_metadata("exit_code", serde_json::json!(out.status.code().unwrap_or(-1)))
                }
            }
            Err(e) => ToolResult::error(format!("Failed to execute command: {}", e)),
        }
    }
}

pub struct ReadFileTool;

#[async_trait::async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "Read contents of a file"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read".to_string(),
            description: "Read file contents. Shows file path and line count.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    },
                    "offset": {
                        "type": "number",
                        "description": "Line number to start reading from (1-indexed)"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum number of lines to read"
                    }
                },
                "required": ["file_path"]
            }),
            category: ToolCategory::FileSystem,
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let file_path = match input.get("file_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing 'file_path' parameter"),
        };

        let offset = input.get("offset").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
        let limit = input.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);

        match tokio::fs::read_to_string(file_path).await {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let total_lines = lines.len();
                
                let start = offset.saturating_sub(1).min(lines.len());
                let end = limit.map(|l| (start + l).min(lines.len())).unwrap_or(lines.len());
                
                let selected_lines = &lines[start..end];
                let result = selected_lines.join("\n");

                ToolResult::success(format!(
                    "File: {}\nLines: {}-{} of {}\n\n{}",
                    file_path,
                    start + 1,
                        end,
                    total_lines,
                    result
                ))
                .with_metadata("line_count", serde_json::json!(end - start))
                .with_metadata("total_lines", serde_json::json!(total_lines))
            }
            Err(e) => ToolResult::error(format!("Failed to read file: {}", e)),
        }
    }
}

pub struct WriteFileTool;

#[async_trait::async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write"
    }

    fn description(&self) -> &str {
        "Create or overwrite a file with content"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "write".to_string(),
            description: "Create a new file or overwrite existing file.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path where the file will be created"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    }
                },
                "required": ["file_path", "content"]
            }),
            category: ToolCategory::FileSystem,
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let file_path = match input.get("file_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing 'file_path' parameter"),
        };

        let content = match input.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ToolResult::error("Missing 'content' parameter"),
        };

        match tokio::fs::write(file_path, content).await {
            Ok(_) => ToolResult::success(format!("File '{}' written successfully", file_path))
                .with_metadata("bytes_written", serde_json::json!(content.len())),
            Err(e) => ToolResult::error(format!("Failed to write file: {}", e)),
        }
    }
}

pub struct GlobTool;

#[async_trait::async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Search for files matching a glob pattern"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "glob".to_string(),
            description: "Find files matching a glob pattern.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern (e.g., '**/*.rs')"
                    },
                    "root": {
                        "type": "string",
                        "description": "Root directory to search from"
                    }
                },
                "required": ["pattern"]
            }),
            category: ToolCategory::Search,
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let pattern = match input.get("pattern").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing 'pattern' parameter"),
        };

        let root = input.get("root").and_then(|v| v.as_str()).unwrap_or(".");

        let mut matches = Vec::new();
        for entry in glob::glob(&format!("{}/{}", root, pattern)).unwrap() {
            if let Ok(path) = entry {
                matches.push(path.display().to_string());
            }
        }

        matches.sort();

        ToolResult::success(format!("Found {} file(s):\n\n{}", matches.len(), matches.join("\n")))
            .with_metadata("match_count", serde_json::json!(matches.len()))
            .with_metadata("matches", serde_json::json!(matches))
    }
}

pub struct GrepTool;

#[async_trait::async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for text patterns in files"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "grep".to_string(),
            description: "Search for a pattern in files.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "Path to search in"
                    },
                    "file_pattern": {
                        "type": "string",
                        "description": "File pattern filter (e.g., '*.rs')"
                    },
                    "context": {
                        "type": "number",
                        "description": "Number of context lines to show"
                    }
                },
                "required": ["pattern", "path"]
            }),
            category: ToolCategory::Search,
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let pattern = match input.get("pattern").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing 'pattern' parameter"),
        };

        let path = match input.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing 'path' parameter"),
        };

        let file_pattern = input.get("file_pattern").and_then(|v| v.as_str());
        let context = input.get("context").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let regex = match regex::Regex::new(pattern) {
            Ok(r) => r,
            Err(e) => return ToolResult::error(format!("Invalid regex: {}", e)),
        };

        let mut matches = Vec::new();

        let walker = walkdir::WalkDir::new(path)
            .into_iter()
            .filter_entry(|e| {
                if let Some(fp) = file_pattern {
                    !glob::Pattern::new(fp)
                        .map(|p| p.matches(&e.file_name().to_string_lossy()))
                        .unwrap_or(false)
                } else {
                    true
                }
            })
            .filter_map(|e| e.ok());

        for entry in walker {
            if entry.file_type().is_file() {
                if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                    for (line_num, line) in content.lines().enumerate() {
                        if regex.is_match(line) {
                            let start = line_num.saturating_sub(context);
                            let end = (line_num + context + 1).min(content.lines().count());
                            matches.push(format!(
                                "{}:{}: {}",
                                entry.path().display(),
                                line_num + 1,
                                line
                            ));
                        }
                    }
                }
            }
        }

        if matches.is_empty() {
            return ToolResult::success("No matches found");
        }

        ToolResult::success(format!("Found {} match(es):\n\n{}", matches.len(), matches.join("\n")))
            .with_metadata("match_count", serde_json::json!(matches.len()))
    }
}

pub struct WebSearchTool;

#[async_trait::async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "web_search".to_string(),
            description: "Search the web for information using DuckDuckGo.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "num_results": {
                        "type": "number",
                        "description": "Number of results to return (default: 5)"
                    }
                },
                "required": ["query"]
            }),
            category: ToolCategory::Web,
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let query = match input.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return ToolResult::error("Missing 'query' parameter"),
        };

        let num_results = input.get("num_results").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

        let url = format!(
            "https://duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; Uclaw/1.0)")
            .build()
            .map_err(|e| ToolResult::error(format!("HTTP client error: {}", e)))
            .unwrap();

        match client.get(&url).send().await {
            Ok(response) => {
                match response.text().await {
                    Ok(html) => {
                        let mut results = Vec::new();
                        for line in html.lines() {
                            if line.contains("result__snippet") {
                                let clean = regex::Regex::new(r"<[^>]+>")
                                    .unwrap()
                                    .replace_all(line, "")
                                    .to_string();
                                if !clean.trim().is_empty() {
                                    results.push(clean.trim().to_string());
                                }
                            }
                            if results.len() >= num_results {
                                break;
                            }
                        }
                        ToolResult::success(format!(
                            "Search results for '{}':\n\n{}",
                            query,
                            results.join("\n\n")
                        ))
                    }
                    Err(e) => ToolResult::error(format!("Failed to read response: {}", e)),
                }
            }
            Err(e) => ToolResult::error(format!("Search failed: {}", e)),
        }
    }
}

pub struct WebFetchTool;

#[async_trait::async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch content from a URL"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "web_fetch".to_string(),
            description: "Fetch and parse content from a URL.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to fetch"
                    },
                    "max_length": {
                        "type": "number",
                        "description": "Maximum content length to return"
                    }
                },
                "required": ["url"]
            }),
            category: ToolCategory::Web,
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let url = match input.get("url").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => return ToolResult::error("Missing 'url' parameter"),
        };

        let max_length = input.get("max_length").and_then(|v| v.as_u64()).unwrap_or(10000) as usize;

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; Uclaw/1.0)")
            .build()
            .map_err(|e| ToolResult::error(format!("HTTP client error: {}", e)))
            .unwrap();

        match client.get(url).send().await {
            Ok(response) => {
                match response.text().await {
                    Ok(text) => {
                        let truncated = if text.len() > max_length {
                            format!("{}...\n\n[Truncated {} chars]", &text[..max_length], text.len() - max_length)
                        } else {
                            text
                        };
                        ToolResult::success(truncated)
                            .with_metadata("content_length", serde_json::json!(text.len()))
                            .with_metadata("url", serde_json::json!(url))
                    }
                    Err(e) => ToolResult::error(format!("Failed to read response: {}", e)),
                }
            }
            Err(e) => ToolResult::error(format!("Fetch failed: {}", e)),
        }
    }
}

pub async fn register_default_tools(registry: &ToolRegistry) {
    registry.register(Arc::new(BashTool)).await;
    registry.register(Arc::new(ReadFileTool)).await;
    registry.register(Arc::new(WriteFileTool)).await;
    registry.register(Arc::new(GlobTool)).await;
    registry.register(Arc::new(GrepTool)).await;
    registry.register(Arc::new(WebSearchTool)).await;
    registry.register(Arc::new(WebFetchTool)).await;
}
