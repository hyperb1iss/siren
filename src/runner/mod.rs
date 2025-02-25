//! Tool execution and runner system

use std::path::PathBuf;
use std::sync::Arc;

use futures::future;
use tokio::task;

use crate::errors::ToolError;
use crate::models::tools::ToolConfig;
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
        tools: Vec<Arc<dyn LintTool>>,
        files: &[PathBuf],
        config: &ToolConfig,
    ) -> Vec<Result<LintResult, ToolError>> {
        let mut handles = Vec::new();

        // Spawn a task for each tool
        for tool in tools {
            // Clone the necessary data for the task
            let files = files.to_vec();
            let config = config.clone();
            let tool = tool.clone();

            // Spawn a task to run the tool
            let handle = task::spawn_blocking(move || {
                // Actually execute the tool on the files
                tool.execute(&files, &config)
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
        self.run_tools(tools, files, config).await
    }

    /// Get tools of a specific type and run them
    pub async fn run_tools_of_type(
        &self,
        tool_type: ToolType,
        files: &[PathBuf],
        config: &ToolConfig,
    ) -> Vec<Result<LintResult, ToolError>> {
        let tools = self.registry.get_tools_by_type(tool_type);
        self.run_tools(tools, files, config).await
    }
}
