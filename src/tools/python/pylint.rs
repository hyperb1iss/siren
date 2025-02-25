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

        // Regex to match PyLint output format
        // Format: file:line:column: [code] message
        let regex =
            Regex::new(r"(?m)^(.+):(\d+):(\d+):\s+\[(\w+)(?:\((\w+)\))?\]\s+(.+)$").unwrap();

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
            let code_type = capture.get(4).unwrap().as_str();
            let code = capture.get(5).map(|m| m.as_str().to_string());
            let message = capture.get(6).unwrap().as_str();

            // Determine severity based on code type
            let severity = match code_type {
                "E" | "F" => IssueSeverity::Error,
                "W" => IssueSeverity::Warning,
                "C" => IssueSeverity::Style,
                "R" => IssueSeverity::Info,
                _ => IssueSeverity::Warning,
            };

            // Create a PathBuf for the file
            let file_path = PathBuf::from(file_str);

            issues.push(LintIssue {
                severity,
                message: message.to_string(),
                file: Some(file_path),
                line: Some(line),
                column: Some(column),
                code: code.map(|c| format!("{}({})", code_type, c)),
                fix_available: false, // PyLint doesn't provide auto-fixes
            });
        }

        issues
    }

    /// Run pylint on a file to check for issues
    fn check_file(
        &self,
        file: &Path,
        config: &ModelsToolConfig,
    ) -> Result<Vec<LintIssue>, ToolError> {
        let mut command = Command::new("pylint");
        command.arg("--output-format=text");
        command.arg("--score=n");
        command.arg("--reports=n");

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add the file to check
        command.arg(file);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute pylint: {}", e),
        })?;

        // Parse the output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let issues = self.parse_output(&stdout);

        Ok(issues)
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
        let mut issues = Vec::new();

        // Process each file
        for file in files {
            if !self.can_handle(file) {
                continue;
            }

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
        utils::is_command_available("pylint")
    }

    fn version(&self) -> Option<String> {
        utils::get_command_version("pylint", &["--version"])
    }

    fn priority(&self) -> usize {
        self.base.priority
    }
}
