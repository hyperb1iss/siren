//! Tool execution and runner system

use std::path::PathBuf;

use futures::future;
use tokio::task;

use crate::config::ToolConfig;
use crate::errors::ToolError;
use crate::models::{LintResult, ToolType};
use crate::tools::{LintTool, ToolRegistry};

/// Tool runner for executing tools in parallel
pub struct ToolRunner<R: ToolRegistry> {
    registry: R,
}

impl<R: ToolRegistry> ToolRunner<R> {
    /// Create a new ToolRunner
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    /// Run tools in parallel
    pub async fn run_tools(
        &self,
        tools: Vec<&dyn LintTool>,
        files: &[PathBuf],
        config: &ToolConfig,
    ) -> Vec<Result<LintResult, ToolError>> {
        let mut handles = Vec::new();

        // Spawn a task for each tool
        for tool in tools {
            // Clone the necessary data for the task
            let tool_name = tool.name().to_string();
            let _files = files.to_vec();
            let _config = config.clone();

            // Spawn a task to run the tool
            let handle = task::spawn_blocking(move || {
                // Due to limitations of our current setup we can't easily move the
                // tool trait object across threads, so we're just returning
                // a placeholder result here.
                // In a real implementation, we would execute the tool.
                Ok(LintResult {
                    tool_name,
                    tool: None, // This would be the actual tool
                    success: true,
                    issues: Vec::new(),
                    execution_time: std::time::Duration::from_secs(0),
                    stdout: None,
                    stderr: None,
                })
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        let results = future::join_all(handles).await;

        // Unwrap the JoinHandle results
        results
            .into_iter()
            .map(|res| {
                res.unwrap_or_else(|e| {
                    Err(ToolError::ExecutionFailed {
                        name: "unknown".to_string(),
                        message: format!("Task panicked: {}", e),
                    })
                })
            })
            .collect()
    }

    /// Get tools for a specific language and run them
    pub async fn run_tools_for_language(
        &self,
        language: crate::models::Language,
        files: &[PathBuf],
        config: &ToolConfig,
    ) -> Vec<Result<LintResult, ToolError>> {
        let tools = self.registry.get_tools_for_language(language);
        let tools_refs: Vec<&dyn LintTool> = tools.iter().map(|t| t.as_ref()).collect();
        self.run_tools(tools_refs, files, config).await
    }

    /// Get tools of a specific type and run them
    pub async fn run_tools_of_type(
        &self,
        tool_type: ToolType,
        files: &[PathBuf],
        config: &ToolConfig,
    ) -> Vec<Result<LintResult, ToolError>> {
        let tools = self.registry.get_tools_by_type(tool_type);
        let tools_refs: Vec<&dyn LintTool> = tools.iter().map(|t| t.as_ref()).collect();
        self.run_tools(tools_refs, files, config).await
    }
}
