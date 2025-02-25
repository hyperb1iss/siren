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
                language: Language::Python,
            },
        }
    }

    /// Run black on a file to check if it needs formatting
    fn check_file(
        &self,
        file: &Path,
        config: &ModelsToolConfig,
    ) -> Result<Vec<LintIssue>, ToolError> {
        let mut command = Command::new("black");
        command.arg("--check");
        command.arg("--quiet");

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

        // Add the file to format
        command.arg(file);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute black: {}", e),
        })?;

        // Parse the output
        if output.status.success() {
            // No formatting issues
            Ok(Vec::new())
        } else {
            // File needs formatting
            Ok(vec![LintIssue {
                severity: IssueSeverity::Style,
                message: "File needs formatting".to_string(),
                file: Some(file.to_path_buf()),
                line: None,
                column: None,
                code: None,
                fix_available: true,
            }])
        }
    }

    /// Fix formatting issues with black
    fn fix_file(&self, file: &Path, config: &ModelsToolConfig) -> Result<(), ToolError> {
        let mut command = Command::new("black");
        command.arg("--quiet");

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

        // Add the file to format
        command.arg(file);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute black: {}", e),
        })?;

        if !output.status.success() {
            return Err(ToolError::ExecutionFailed {
                name: self.name().to_string(),
                message: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(())
    }
}

impl LintTool for Black {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn can_handle(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension() {
            ext == "py" || ext == "pyi" || ext == "pyx"
        } else {
            false
        }
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

        // Process each file
        for file in files {
            if !self.can_handle(file) {
                continue;
            }

            if fix_mode {
                // Fix the file
                self.fix_file(file, config)?
            } else {
                // Check the file
                match self.check_file(file, config) {
                    Ok(file_issues) => {
                        issues.extend(file_issues);
                    }
                    Err(e) => {
                        return Err(e);
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
                language: self.language(),
                available: self.is_available(),
                version: self.version(),
                description: self.description().to_string(),
            }),
            success: issues.is_empty(),
            issues,
            execution_time,
            stdout: None,
            stderr: None,
        })
    }

    fn tool_type(&self) -> ToolType {
        self.base.tool_type
    }

    fn language(&self) -> Language {
        self.base.language
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
