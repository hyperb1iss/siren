//! PyLint linter for Python

use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::errors::ToolError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::{IssueSeverity, Language, LintIssue, LintResult, ToolInfo, ToolType};
use crate::tools::{LintTool, ToolBase};
use crate::utils;

/// PyLint linter for Python
pub struct PyLint {
    base: ToolBase,
}

impl PyLint {
    /// Create a new PyLint linter
    pub fn new() -> Self {
        Self {
            base: ToolBase {
                name: "pylint".to_string(),
                description: "A Python static code analysis tool".to_string(),
                tool_type: ToolType::Linter,
                language: Language::Python,
                priority: 5,
            },
        }
    }

    /// Parse pylint output to extract issues
    fn parse_output(&self, output: &str) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Track current module and file for module-level patterns
        let mut _current_module: Option<String> = None;

        // Pattern for module headers
        let module_regex = Regex::new(r"(?m)^\*+\s+Module\s+(\w+)").unwrap();

        // Primary pattern for pylint issues
        // Format: "incidents.py:1:0: C0114: Missing module docstring (missing-module-docstring)"
        let issue_regex =
            Regex::new(r"(?m)^([^:]+):(\d+):(\d+): ([A-Z]\d+): (.+?) \(([^)]+)\)").unwrap();

        // Process line by line
        for line in output.lines() {
            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Check for module header
            if let Some(cap) = module_regex.captures(line) {
                _current_module = Some(cap[1].to_string());
                continue;
            }

            // Try to match issue pattern
            if let Some(cap) = issue_regex.captures(line) {
                let file_str = cap.get(1).unwrap().as_str();
                let line_num = cap.get(2).unwrap().as_str().parse::<usize>().unwrap_or(0);
                let column = cap.get(3).unwrap().as_str().parse::<usize>().unwrap_or(0);
                let code = cap.get(4).map(|m| m.as_str().to_string());
                let message = cap.get(5).unwrap().as_str();
                let error_type = cap.get(6).unwrap().as_str();

                // Determine severity based on code first letter
                let severity = if let Some(code_str) = &code {
                    match code_str.chars().next() {
                        Some('E') | Some('F') => IssueSeverity::Error,
                        Some('W') => IssueSeverity::Warning,
                        Some('C') => IssueSeverity::Style,
                        Some('R') => IssueSeverity::Info,
                        _ => IssueSeverity::Warning,
                    }
                } else {
                    IssueSeverity::Warning
                };

                // Create a PathBuf for the file
                let file_path = PathBuf::from(file_str);

                issues.push(LintIssue {
                    severity,
                    message: format!("{} ({})", message, error_type),
                    file: Some(file_path),
                    line: Some(line_num),
                    column: Some(column),
                    code,
                    fix_available: false,
                });
            }
        }

        issues
    }

    /// Run pylint on multiple files to check for issues
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

        let mut command = Command::new("pylint");
        command.arg("--output-format=text");
        command.arg("--score=n");
        command.arg("--reports=n");

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add all the files to check
        for file in files_to_check {
            command.arg(file);
        }

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute pylint: {}", e),
        })?;

        // Parse the output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let issues = self.parse_output(&stdout);

        Ok((issues, stdout, stderr))
    }
}

impl LintTool for PyLint {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn can_handle(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension() {
            ext == "py"
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

        // Run pylint once for all files
        let (issues, stdout, stderr) = self.check_files(files, config)?;

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
        utils::is_command_available("pylint")
    }

    fn version(&self) -> Option<String> {
        utils::get_command_version("pylint", &["--version"])
    }

    fn priority(&self) -> usize {
        self.base.priority
    }
}
