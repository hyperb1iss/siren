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

    /// Run ruff on multiple files to check for issues
    fn check_files(
        &self,
        files: &[PathBuf],
        config: &ModelsToolConfig,
    ) -> Result<(Vec<LintIssue>, String, String), ToolError> {
        // Skip if no files can be handled
        let files_to_check: Vec<&Path> = files
            .iter()
            .filter(|file| self.can_handle(file))
            .map(|file| file.as_path())
            .collect();

        if files_to_check.is_empty() {
            return Ok((Vec::new(), String::new(), String::new()));
        }

        let mut command = Command::new("ruff");
        command.arg("check");

        // Set default line length to 88
        let mut has_line_length = false;

        // Add extra arguments
        for arg in &config.extra_args {
            if arg.starts_with("--line-length") {
                has_line_length = true;
            }
            command.arg(arg);
        }

        // Add default line length if not specified
        if !has_line_length {
            command.arg("--line-length=88");
        }

        // Add output format
        command.arg("--output-format=full");

        // Add all the files to check
        for file in files_to_check {
            command.arg(file);
        }

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute ruff: {}", e),
        })?;

        // Parse the output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let issues = self.parse_output(&stdout);

        Ok((issues, stdout, stderr))
    }

    /// Run ruff on multiple files to fix issues
    fn fix_files(
        &self,
        files: &[PathBuf],
        config: &ModelsToolConfig,
    ) -> Result<(String, String), ToolError> {
        // Skip if no files can be handled
        let files_to_check: Vec<&Path> = files
            .iter()
            .filter(|file| self.can_handle(file))
            .map(|file| file.as_path())
            .collect();

        if files_to_check.is_empty() {
            return Ok((String::new(), String::new()));
        }

        let mut command = Command::new("ruff");
        command.arg("check");
        command.arg("--fix");

        // Set default line length to 88
        let mut has_line_length = false;

        // Add extra arguments
        for arg in &config.extra_args {
            if arg.starts_with("--line-length") {
                has_line_length = true;
            }
            command.arg(arg);
        }

        // Add default line length if not specified
        if !has_line_length {
            command.arg("--line-length=88");
        }

        // Add output format
        command.arg("--output-format=full");

        // Add all the files to fix
        for file in files_to_check {
            command.arg(file);
        }

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute ruff: {}", e),
        })?;

        // Parse the output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok((stdout, stderr))
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

        // Check if we should fix issues
        let fix_mode = config.auto_fix;

        let (issues, stdout, stderr) = if fix_mode {
            // Fix all files in one go
            let (fix_stdout, fix_stderr) = self.fix_files(files, config)?;
            // After fixing, run check to get remaining issues
            let (check_issues, check_stdout, check_stderr) = self.check_files(files, config)?;

            // Combine stdout and stderr
            let combined_stdout = if fix_stdout.is_empty() {
                check_stdout
            } else if check_stdout.is_empty() {
                fix_stdout
            } else {
                format!("{}\n\n{}", fix_stdout, check_stdout)
            };

            let combined_stderr = if fix_stderr.is_empty() {
                check_stderr
            } else if check_stderr.is_empty() {
                fix_stderr
            } else {
                format!("{}\n\n{}", fix_stderr, check_stderr)
            };

            (check_issues, combined_stdout, combined_stderr)
        } else {
            // Just check all files in one go
            self.check_files(files, config)?
        };

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
            success: true, // Tool executed successfully even if issues were found
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
}
