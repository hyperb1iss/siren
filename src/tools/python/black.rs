//! Black formatter for Python

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::errors::ToolError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::{IssueSeverity, Language, LintIssue, LintResult, ToolInfo, ToolType};
use crate::tools::{LintTool, ToolBase};
use crate::utils;

/// Black formatter for Python
pub struct Black {
    base: ToolBase,
}

impl Black {
    /// Create a new Black formatter
    pub fn new() -> Self {
        Self {
            base: ToolBase {
                name: "black".to_string(),
                description: "The uncompromising Python code formatter".to_string(),
                tool_type: ToolType::Formatter,
                languages: vec![Language::Python],
            },
        }
    }
}

impl LintTool for Black {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn can_handle(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension() {
            if ext == "py" || ext == "pyi" || ext == "pyx" {
                // Check if the file is in a valid Python package
                if let Some(parent) = file_path.parent() {
                    return utils::is_valid_python_package(parent);
                }
                return true;
            }
        }
        false
    }

    fn execute(
        &self,
        files: &[PathBuf],
        config: &ModelsToolConfig,
    ) -> Result<LintResult, ToolError> {
        let start = Instant::now();
        let mut issues = Vec::new();

        // Check if we should fix issues
        let fix_mode = config.auto_fix;

        // Filter files that can be handled by black
        let files_to_process: Vec<PathBuf> = files
            .iter()
            .filter(|file| self.can_handle(file))
            .cloned()
            .collect();

        if files_to_process.is_empty() {
            return Ok(LintResult {
                tool_name: self.name().to_string(),
                tool: Some(ToolInfo {
                    name: self.name().to_string(),
                    tool_type: self.tool_type(),
                    languages: self.languages(),
                    available: self.is_available(),
                    version: self.version(),
                    description: self.description().to_string(),
                }),
                success: true,
                issues: Vec::new(),
                execution_time: start.elapsed(),
                stdout: None,
                stderr: None,
            });
        }

        // Optimize paths by grouping by directory when possible
        let optimized_paths = utils::optimize_paths_for_tools(&files_to_process);

        // Create a single command for all files
        let mut command = Command::new("black");

        // Add common flags
        command.arg("--quiet");

        // Add check mode if not fixing
        if !fix_mode {
            command.arg("--check");
        }

        // Add line length if specified in extra args
        // Look for --line-length in extra_args
        let has_line_length = config
            .extra_args
            .iter()
            .any(|arg| arg.starts_with("--line-length"));
        if !has_line_length {
            // Default line length for black
            command.arg("--line-length").arg("88");
        }

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add all paths to process
        for path in &optimized_paths {
            command.arg(path);
        }

        // Log the command
        utils::log_command(&command);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute black: {}", e),
        })?;

        // Parse the output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // If in check mode and command failed, it means formatting issues were found
        if !fix_mode && !output.status.success() {
            // Parse the output to find which files have formatting issues
            for line in stdout.lines().chain(stderr.lines()) {
                if line.contains("would be reformatted") {
                    if let Some(file_path) = line.split_whitespace().next() {
                        issues.push(LintIssue {
                            severity: IssueSeverity::Style,
                            message: "File needs formatting".to_string(),
                            file: Some(PathBuf::from(file_path)),
                            line: None,
                            column: None,
                            code: None,
                            fix_available: true,
                        });
                    }
                }
            }
        }

        let execution_time = start.elapsed();

        Ok(LintResult {
            tool_name: self.name().to_string(),
            tool: Some(ToolInfo {
                name: self.name().to_string(),
                tool_type: self.tool_type(),
                languages: self.languages(),
                available: self.is_available(),
                version: self.version(),
                description: self.description().to_string(),
            }),
            success: output.status.success(),
            issues,
            execution_time,
            stdout: if stdout.is_empty() {
                None
            } else {
                Some(stdout)
            },
            stderr: if stderr.is_empty() {
                None
            } else {
                Some(stderr)
            },
        })
    }

    fn tool_type(&self) -> ToolType {
        self.base.tool_type
    }

    fn languages(&self) -> Vec<Language> {
        self.base.languages.clone()
    }

    fn description(&self) -> &str {
        &self.base.description
    }

    fn is_available(&self) -> bool {
        utils::is_command_available("black")
    }

    fn version(&self) -> Option<String> {
        utils::get_command_version("black", &["--version"])
    }
}
