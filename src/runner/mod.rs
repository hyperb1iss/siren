//! Tool execution and runner system

use std::path::PathBuf;
use std::sync::Arc;

use futures::future;
use tokio::task;

use crate::errors::ToolError;
use crate::models::tools::ToolConfig;
use crate::models::LintResult;
use crate::tools::LintTool;

/// Tool runner for executing tools in parallel
pub struct ToolRunner {}

impl Default for ToolRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRunner {
    /// Create a new ToolRunner
    pub fn new() -> Self {
        Self {}
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
            .map(|r| {
                r.unwrap_or_else(|e| {
                    Err(ToolError::ExecutionFailed {
                        name: "unknown".to_string(),
                        message: format!("Task join error: {}", e),
                    })
                })
            })
            .collect()
    }

    /// Run tools in parallel with specific paths for each tool
    pub async fn run_tools_with_specific_paths(
        &self,
        tools: Vec<Arc<dyn LintTool>>,
        files_per_tool: Vec<Vec<PathBuf>>,
        config: &ToolConfig,
    ) -> Vec<Result<LintResult, ToolError>> {
        let mut handles = Vec::new();

        // Ensure we have the same number of tools and file sets
        assert_eq!(
            tools.len(),
            files_per_tool.len(),
            "Number of tools must match number of file sets"
        );

        // Spawn a task for each tool with its specific files
        for (i, tool) in tools.into_iter().enumerate() {
            // Get the specific files for this tool
            let files = files_per_tool[i].clone();
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
            .map(|r| {
                r.unwrap_or_else(|e| {
                    Err(ToolError::ExecutionFailed {
                        name: "unknown".to_string(),
                        message: format!("Task join error: {}", e),
                    })
                })
            })
            .collect()
    }
}
