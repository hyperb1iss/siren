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
}
