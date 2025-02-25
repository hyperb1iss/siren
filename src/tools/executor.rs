use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use log::{debug, error};
use tokio::task;

use crate::errors::ToolError;
use crate::models::{Language, LintResult, ToolConfig, ToolInfo, ToolType};
use crate::tools::{LintTool, ToolRegistry};

/// Tool executor for running tools in parallel
pub struct ToolExecutor<R: ToolRegistry> {
    /// Tool registry
    registry: R,
}

impl<R: ToolRegistry> ToolExecutor<R> {
    /// Create a new tool executor
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    /// Run a specific tool on files
    pub async fn run_tool(
        &self,
        tool: Arc<dyn LintTool>,
        files: &[PathBuf],
        config: &ToolConfig,
    ) -> Result<LintResult, ToolError> {
        // If tool is not available, return error
        if !tool.is_available() {
            return Err(ToolError::NotFound(tool.name().to_string()));
        }

        // If no files to process, return empty success result
        if files.is_empty() {
            debug!("No files to process for tool {}", tool.name());
            return Ok(LintResult {
                tool_name: tool.name().to_string(),
                tool: Some(ToolInfo {
                    name: tool.name().to_string(),
                    tool_type: tool.tool_type(),
                    language: tool.language(),
                    available: tool.is_available(),
                    version: tool.version(),
                    description: tool.description().to_string(),
                }),
                success: true,
                issues: Vec::new(),
                execution_time: std::time::Duration::from_secs(0),
                stdout: None,
                stderr: None,
            });
        }

        debug!("Running tool {} on {} files", tool.name(), files.len());

        // Filter files that this tool can handle
        let files: Vec<PathBuf> = files
            .iter()
            .filter(|f| tool.can_handle(f))
            .cloned()
            .collect();

        if files.is_empty() {
            debug!("No files match tool {}", tool.name());
            return Ok(LintResult {
                tool_name: tool.name().to_string(),
                tool: Some(ToolInfo {
                    name: tool.name().to_string(),
                    tool_type: tool.tool_type(),
                    language: tool.language(),
                    available: tool.is_available(),
                    version: tool.version(),
                    description: tool.description().to_string(),
                }),
                success: true,
                issues: Vec::new(),
                execution_time: std::time::Duration::from_secs(0),
                stdout: None,
                stderr: None,
            });
        }

        // Execute tool in a blocking task to avoid blocking the async executor
        let tool_clone = tool.clone();
        let start = Instant::now();
        let config_clone = config.clone(); // Clone the config to avoid borrowing issues
        let result = task::spawn_blocking(move || tool_clone.execute(&files, &config_clone))
            .await
            .unwrap_or_else(|e| {
                error!("Failed to execute tool {}: {}", tool.name(), e);
                Err(ToolError::ExecutionFailed {
                    name: tool.name().to_string(),
                    message: e.to_string(),
                })
            })?;

        debug!(
            "Tool {} completed in {:?} with {} issues",
            tool.name(),
            start.elapsed(),
            result.issues.len()
        );

        Ok(result)
    }

    /// Run tools for a specific language
    pub async fn run_tools_for_language(
        &self,
        language: Language,
        files: &[PathBuf],
        config: &ToolConfig,
    ) -> Vec<Result<LintResult, ToolError>> {
        let tools = self.registry.get_tools_for_language(language);

        if tools.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();

        for tool in tools {
            match self.run_tool(tool.clone(), files, config).await {
                Ok(result) => results.push(Ok(result)),
                Err(err) => results.push(Err(err)),
            }
        }

        results
    }

    /// Run tools for a specific language and type
    pub async fn run_tools_for_language_and_type(
        &self,
        language: Language,
        tool_type: ToolType,
        files: &[PathBuf],
        config: &ToolConfig,
    ) -> Vec<Result<LintResult, ToolError>> {
        let tools = self
            .registry
            .get_tools_for_language_and_type(language, tool_type);

        if tools.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();

        for tool in tools {
            match self.run_tool(tool.clone(), files, config).await {
                Ok(result) => results.push(Ok(result)),
                Err(err) => results.push(Err(err)),
            }
        }

        results
    }

    /// Run tools for a list of files (auto-detect languages and tools)
    pub async fn run_tools_for_files(
        &self,
        files: &[PathBuf],
        config: &ToolConfig,
    ) -> Vec<Result<LintResult, ToolError>> {
        // Group files by language for more efficient processing
        let mut files_by_language: std::collections::HashMap<Language, Vec<PathBuf>> =
            std::collections::HashMap::new();

        for file in files {
            if let Some(language) = Language::from_path(file) {
                files_by_language
                    .entry(language)
                    .or_default()
                    .push(file.clone());
            }
        }

        let mut results = Vec::new();

        // Process each language in parallel
        for (language, language_files) in files_by_language {
            debug!(
                "Running tools for language {} with {} files",
                language,
                language_files.len()
            );

            let language_results = self
                .run_tools_for_language(language, &language_files, config)
                .await;
            results.extend(language_results);
        }

        results
    }

    /// Run all tools matching a filter function
    pub async fn run_filtered_tools<F>(
        &self,
        filter: F,
        files: &[PathBuf],
        config: &ToolConfig,
    ) -> Vec<Result<LintResult, ToolError>>
    where
        F: Fn(&dyn LintTool) -> bool,
    {
        let tools: Vec<Arc<dyn LintTool>> = self
            .registry
            .get_all_tools()
            .into_iter()
            .filter(|tool| filter(tool.as_ref()))
            .collect();

        let mut results = Vec::new();

        for tool in tools {
            match self.run_tool(tool.clone(), files, config).await {
                Ok(result) => results.push(Ok(result)),
                Err(err) => results.push(Err(err)),
            }
        }

        results
    }

    /// Group files by language
    pub fn group_files_by_language(
        &self,
        files: &[PathBuf],
    ) -> std::collections::HashMap<Language, Vec<PathBuf>> {
        let mut result: std::collections::HashMap<Language, Vec<PathBuf>> =
            std::collections::HashMap::new();

        for file in files {
            if let Some(language) = Language::from_path(file) {
                result.entry(language).or_default().push(file.clone());
            }
        }

        result
    }
}
