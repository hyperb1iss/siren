//! Ruff linter for Python

use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::errors::ToolError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::{IssueSeverity, Language, LintIssue, LintResult, ToolInfo, ToolType};
use crate::tools::{LintTool, ToolBase};
use crate::utils;

/// Ruff linter for Python
pub struct Ruff {
    base: ToolBase,
}

impl Ruff {
    /// Create a new Ruff linter
    pub fn new() -> Self {
        Self {
            base: ToolBase {
                name: "ruff".to_string(),
                description: "An extremely fast Python linter, written in Rust".to_string(),
                tool_type: ToolType::Linter,
                language: Language::Python,
                priority: 8,
            },
        }
    }

    /// Parse ruff output to extract issues
    fn parse_output(&self, output: &str) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Regex to match Ruff output format
        // Format: file:line:column: error code: message
        let regex = Regex::new(r"(?m)^(.+):(\d+):(\d+):\s+(\w+\d+):\s+(.+)$").unwrap();

        for capture in regex.captures_iter(output) {
            let file_str = capture.get(1).unwrap().as_str();
            let line = capture
                .get(2)
                .unwrap()
                .as_str()
                .parse::<usize>()
                .unwrap_or(0);
            let column = capture
                .get(3)
                .unwrap()
                .as_str()
                .parse::<usize>()
                .unwrap_or(0);
            let code = capture.get(4).unwrap().as_str();
            let message = capture.get(5).unwrap().as_str();

            // Determine severity based on code prefix
            let severity = if code.starts_with('E') || code.starts_with('F') {
                IssueSeverity::Error
            } else if code.starts_with('W') {
                IssueSeverity::Warning
            } else {
                IssueSeverity::Style
            };

            // Create a PathBuf for the file
            let file_path = PathBuf::from(file_str);

            issues.push(LintIssue {
                severity,
                message: format!("{}: {}", code, message),
                file: Some(file_path),
                line: Some(line),
                column: Some(column),
                code: Some(code.to_string()),
                fix_available: true, // Ruff can fix most issues
            });
        }

        issues
    }

    /// Run ruff on a file to check for issues
    fn check_file(
        &self,
        file: &Path,
        config: &ModelsToolConfig,
    ) -> Result<Vec<LintIssue>, ToolError> {
        let mut command = Command::new("ruff");
        command.arg("check");

        // Add line length if specified in extra args
        // Look for --line-length in extra_args
        let has_line_length = config
            .extra_args
            .iter()
            .any(|arg| arg.starts_with("--line-length"));
        if !has_line_length {
            // Default line length for ruff
            command.arg("--line-length").arg("88");
        }

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add the file to check
        command.arg(file);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute ruff: {}", e),
        })?;

        // Parse the output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let issues = self.parse_output(&stdout);

        Ok(issues)
    }

    /// Fix issues with ruff
    fn fix_file(&self, file: &Path, config: &ModelsToolConfig) -> Result<(), ToolError> {
        let mut command = Command::new("ruff");
        command.arg("check");
        command.arg("--fix");

        // Add line length if specified in extra args
        // Look for --line-length in extra_args
        let has_line_length = config
            .extra_args
            .iter()
            .any(|arg| arg.starts_with("--line-length"));
        if !has_line_length {
            // Default line length for ruff
            command.arg("--line-length").arg("88");
        }

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add the file to fix
        command.arg(file);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute ruff: {}", e),
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

impl LintTool for Ruff {
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
        utils::is_command_available("ruff")
    }

    fn version(&self) -> Option<String> {
        utils::get_command_version("ruff", &["--version"])
    }

    fn priority(&self) -> usize {
        self.base.priority
    }
}
