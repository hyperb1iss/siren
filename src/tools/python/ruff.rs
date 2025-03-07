//! Ruff linter for Python

use log::debug;
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
                languages: vec![Language::Python],
            },
        }
    }

    /// Parse ruff output to extract issues
    fn parse_output(&self, output: &str) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Regex to match Ruff output format when using --output-format=concise
        // Format: file:line:column: error_code [*] message
        let regex = Regex::new(r"(?m)^(.+):(\d+):(\d+):\s+(\w+\d+)(?:\s+\[\*\])?\s+(.+)$").unwrap();

        for capture in regex.captures_iter(output) {
            let file_str = capture.get(1).unwrap().as_str();
            let file_path = PathBuf::from(file_str);

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

            // Check if the issue is fixable (has [*] marker)
            let fix_available =
                output.contains(&format!("{}:{}:{}: {} [*]", file_str, line, column, code));

            issues.push(LintIssue {
                severity,
                message: format!("{}: {}", code, message),
                file: Some(file_path),
                line: Some(line),
                column: Some(column),
                code: Some(code.to_string()),
                fix_available, // Set based on [*] marker
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
        let files_to_check: Vec<PathBuf> = files
            .iter()
            .filter(|file| self.can_handle(file))
            .cloned()
            .collect();

        if files_to_check.is_empty() {
            return Ok((Vec::new(), String::new(), String::new()));
        }

        // Optimize paths by grouping by directory when possible
        let optimized_paths = utils::optimize_paths_for_tools(&files_to_check);

        let mut command = Command::new("ruff");
        command.arg("check");

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add output format
        command.arg("--output-format=concise");

        // Add all the paths to check
        for path in &optimized_paths {
            command.arg(path);
        }

        // Log the command
        utils::log_command(&command);

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
        let files_to_check: Vec<PathBuf> = files
            .iter()
            .filter(|file| self.can_handle(file))
            .cloned()
            .collect();

        if files_to_check.is_empty() {
            return Ok((String::new(), String::new()));
        }

        // Optimize paths by grouping by directory when possible
        let optimized_paths = utils::optimize_paths_for_tools(&files_to_check);

        let mut command = Command::new("ruff");
        command.arg("check");
        command.arg("--fix");

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add output format
        command.arg("--output-format=concise");

        // Add all the paths to fix
        for path in &optimized_paths {
            command.arg(path);
        }

        // Log the command
        utils::log_command(&command);

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
                languages: self.languages(),
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

    fn languages(&self) -> Vec<Language> {
        self.base.languages.clone()
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

/// Ruff formatter for Python
pub struct RuffFormatter {
    base: ToolBase,
}

impl RuffFormatter {
    /// Create a new Ruff formatter
    pub fn new() -> Self {
        Self {
            base: ToolBase {
                name: "ruff_formatter".to_string(),
                description: "An extremely fast Python formatter, written in Rust".to_string(),
                tool_type: ToolType::Formatter,
                languages: vec![Language::Python],
            },
        }
    }

    /// Format files using Ruff
    fn format_files(
        &self,
        files: &[PathBuf],
        config: &ModelsToolConfig,
    ) -> Result<(String, String), ToolError> {
        // Skip if no files can be handled
        let files_to_format: Vec<PathBuf> = files
            .iter()
            .filter(|file| self.can_handle(file))
            .cloned()
            .collect();

        if files_to_format.is_empty() {
            return Ok((String::new(), String::new()));
        }

        // Optimize paths by grouping by directory when possible
        let optimized_paths = utils::optimize_paths_for_tools(&files_to_format);

        let mut command = Command::new("ruff");
        command.arg("format");

        // Add check mode if requested
        if config.check {
            command.arg("--check");
        }

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add all the paths to format
        for path in &optimized_paths {
            command.arg(path);
        }

        // Log the command
        utils::log_command(&command);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute ruff format: {}", e),
        })?;

        // Parse the output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok((stdout, stderr))
    }
}

impl LintTool for RuffFormatter {
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

        // Format files
        let (stdout, stderr) = self.format_files(files, config)?;

        // Parse the output to determine if formatting is needed
        let mut issues = Vec::new();

        // Debug output to help diagnose issues
        debug!("Ruff formatter stdout: {}", stdout);
        debug!("Ruff formatter stderr: {}", stderr);

        // When in check mode, ruff format will output "Would reformat X" for files that need formatting
        if config.check {
            // Extract file paths that would be reformatted
            let would_reformat_regex = Regex::new(r"Would reformat: (.+)").unwrap();
            let files_reformatted_regex = Regex::new(r"(\d+) files? would be reformatted").unwrap();

            // Check if any files would be reformatted
            let mut found_specific_files = false;

            // First check for specific files
            for line in stdout.lines().chain(stderr.lines()) {
                if let Some(captures) = would_reformat_regex.captures(line) {
                    if let Some(file_match) = captures.get(1) {
                        found_specific_files = true;
                        let file_path = PathBuf::from(file_match.as_str());

                        issues.push(LintIssue {
                            severity: IssueSeverity::Style,
                            message: "File needs formatting".to_string(),
                            file: Some(file_path),
                            line: None,
                            column: None,
                            code: None,
                            fix_available: true,
                        });
                    }
                }
            }

            // If we couldn't extract specific files but know formatting is needed
            if !found_specific_files {
                for line in stdout.lines().chain(stderr.lines()) {
                    if let Some(captures) = files_reformatted_regex.captures(line) {
                        if let Some(count_match) = captures.get(1) {
                            if let Ok(count) = count_match.as_str().parse::<usize>() {
                                if count > 0 {
                                    // Add a generic issue for each file
                                    for file in files {
                                        if self.can_handle(file) {
                                            issues.push(LintIssue {
                                                severity: IssueSeverity::Style,
                                                message: "File needs formatting".to_string(),
                                                file: Some(file.clone()),
                                                line: None,
                                                column: None,
                                                code: None,
                                                fix_available: true,
                                            });
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            // If we still haven't found any issues but the command exited with a non-zero status,
            // assume formatting is needed for all files
            if issues.is_empty() && !stdout.is_empty() {
                for file in files {
                    if self.can_handle(file) {
                        issues.push(LintIssue {
                            severity: IssueSeverity::Style,
                            message: "File needs formatting".to_string(),
                            file: Some(file.clone()),
                            line: None,
                            column: None,
                            code: None,
                            fix_available: true,
                        });
                    }
                }
            }
        }

        // Create a result with issues if formatting is needed
        let result = LintResult {
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
            issues,
            execution_time: start.elapsed(),
            stdout: Some(stdout),
            stderr: Some(stderr),
        };

        Ok(result)
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
        utils::is_command_available("ruff")
    }

    fn version(&self) -> Option<String> {
        utils::get_command_version("ruff", &["--version"])
    }
}
