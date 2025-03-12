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

        // Try to parse as JSON
        match serde_json::from_str::<serde_json::Value>(output) {
            Ok(json_value) => {
                if let Some(array) = json_value.as_array() {
                    debug!("Processing {} Ruff diagnostics", array.len());
                    for (i, diagnostic) in array.iter().enumerate() {
                        // Extract required fields
                        let path = diagnostic.get("filename").and_then(|v| v.as_str());
                        let line_num = diagnostic
                            .get("location")
                            .and_then(|v| v.get("row"))
                            .and_then(|v| v.as_u64());
                        let column_num = diagnostic
                            .get("location")
                            .and_then(|v| v.get("column"))
                            .and_then(|v| v.as_u64());
                        let code_str = diagnostic.get("code").and_then(|v| v.as_str());
                        let message_str = diagnostic.get("message").and_then(|v| v.as_str());

                        match (path, line_num, column_num, code_str, message_str) {
                            (
                                Some(path),
                                Some(line_num),
                                Some(column_num),
                                Some(code_str),
                                Some(message_str),
                            ) => {
                                let file_path = PathBuf::from(path);
                                let line = line_num as usize;
                                let column = column_num as usize;

                                // Determine severity based on code prefix
                                let severity =
                                    if code_str.starts_with('E') || code_str.starts_with('F') {
                                        IssueSeverity::Error
                                    } else if code_str.starts_with('W') {
                                        IssueSeverity::Warning
                                    } else {
                                        IssueSeverity::Style
                                    };

                                // Check if the issue is fixable
                                let fix_available = diagnostic.get("fix").is_some();

                                // Get full message with code display if available
                                let mut formatted_message =
                                    format!("{}: {}", code_str, message_str);

                                // Add URL if available
                                if let Some(url) = diagnostic.get("url").and_then(|v| v.as_str()) {
                                    formatted_message.push_str(&format!("\nSee: {}", url));
                                }

                                // Add fix information if available
                                if let Some(fix) = diagnostic.get("fix") {
                                    if let Some(fix_msg) =
                                        fix.get("message").and_then(|v| v.as_str())
                                    {
                                        formatted_message.push_str(&format!("\nFix: {}", fix_msg));
                                    }
                                }

                                debug!(
                                    "Processing diagnostic {}: {} at {}:{}",
                                    i + 1,
                                    code_str,
                                    line,
                                    column
                                );
                                issues.push(LintIssue {
                                    severity,
                                    message: formatted_message,
                                    file: Some(file_path),
                                    line: Some(line),
                                    column: Some(column),
                                    code: Some(code_str.to_string()),
                                    fix_available,
                                });
                            }
                            _ => {
                                debug!(
                                    "Skipping diagnostic {} due to missing required fields: {:?}",
                                    i + 1,
                                    diagnostic
                                );
                            }
                        }
                    }
                } else {
                    debug!("Ruff output is not a JSON array: {}", output);
                }
            }
            Err(e) => {
                debug!(
                    "Failed to parse Ruff output as JSON: {} - Output: {}",
                    e, output
                );
            }
        }

        debug!("Finished parsing {} Ruff issues", issues.len());
        issues
    }

    /// Run ruff on multiple files to check for issues
    fn check_files(
        &self,
        files: &[PathBuf],
        config: &ModelsToolConfig,
    ) -> Result<(Vec<LintIssue>, String, String), ToolError> {
        // Skip if no files to check
        if files.is_empty() {
            return Ok((Vec::new(), String::new(), String::new()));
        }

        // We'll use the files directly - we already did path optimization in the command handler
        let paths_to_check = files;

        let mut command = Command::new("ruff");
        command.arg("check");

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add output format as JSON
        command.arg("--output-format=json");

        // Add all the paths to check
        for path in paths_to_check {
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

        // Log raw output for debugging
        debug!("Ruff raw stdout: {}", stdout);
        debug!("Ruff raw stderr: {}", stderr);

        // If we got a non-zero exit code but no stdout/stderr, this might be a tool failure
        if !output.status.success() && stdout.is_empty() && stderr.is_empty() {
            return Err(ToolError::ExecutionFailed {
                name: self.name().to_string(),
                message: format!("Ruff check failed with exit code: {}", output.status),
            });
        }

        // Parse issues from JSON output
        let issues = self.parse_output(&stdout);

        Ok((issues, stdout, stderr))
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

        // Check files
        let (issues, stdout, stderr) = self.check_files(files, config)?;

        // Debug output to help diagnose issues
        debug!("Ruff found {} issues", issues.len());
        for (i, issue) in issues.iter().enumerate() {
            debug!(
                "Issue {}: {} at {:?}:{:?}",
                i + 1,
                issue.message,
                issue.file,
                issue.line
            );
        }

        // Create a result with all issues found
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
        // Skip if no files to format
        if files.is_empty() {
            return Ok((String::new(), String::new()));
        }

        // We'll use the files directly - we already did path optimization in the command handler
        let paths_to_format = files;

        let mut command = Command::new("ruff");
        command.arg("format");

        // Add check mode if requested
        if config.check {
            command.arg("--check");
            // In check mode, we want to see which files would be reformatted
            command.arg("--diff");
        }

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add all the paths to format
        for path in paths_to_format {
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

        // In check mode, a non-zero exit code means formatting is needed
        if config.check && !output.status.success() {
            debug!(
                "Ruff formatter found formatting issues (exit code: {})",
                output.status
            );
        }

        Ok((stdout, stderr))
    }

    /// Parse formatting output to determine which files need formatting
    fn parse_check_output(&self, stdout: &str, stderr: &str, files: &[PathBuf]) -> Vec<LintIssue> {
        let mut issues = Vec::new();
        let mut found_specific_files = false;

        // First try to find specific files from diff output
        for line in stdout.lines() {
            // diff output starts with "--- " for original file
            if line.starts_with("--- ") {
                if let Some(file_path_str) = line.strip_prefix("--- ") {
                    // Remove "(original)" suffix if present
                    let clean_path = file_path_str.trim_end_matches(" (original)");
                    if !clean_path.starts_with('/') {
                        // Skip /dev/null and other special paths
                        found_specific_files = true;
                        let file_path = PathBuf::from(clean_path);
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
        }

        // If we couldn't find specific files from diff, check for "Would reformat" messages
        if !found_specific_files {
            let would_reformat_regex = Regex::new(r"Would reformat (?:file )?(.+)").unwrap();
            let files_reformatted_regex = Regex::new(r"(\d+) files? would be reformatted").unwrap();

            // Check for specific files
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

            // If still no specific files but we know formatting is needed
            if !found_specific_files {
                for line in stdout.lines().chain(stderr.lines()) {
                    if let Some(captures) = files_reformatted_regex.captures(line) {
                        if let Some(count_match) = captures.get(1) {
                            if let Ok(count) = count_match.as_str().parse::<usize>() {
                                if count > 0 {
                                    // Add an issue for each file that needs formatting
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
        }

        issues
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

        // When in check mode, parse output to find files needing formatting
        if config.check {
            issues = self.parse_check_output(&stdout, &stderr, files);
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
