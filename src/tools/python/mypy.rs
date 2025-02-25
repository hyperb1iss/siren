//! MyPy type checker for Python

use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::errors::ToolError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::{IssueSeverity, Language, LintIssue, LintResult, ToolInfo, ToolType};
use crate::tools::{LintTool, ToolBase};
use crate::utils;

/// MyPy type checker for Python
pub struct MyPy {
    base: ToolBase,
}

impl MyPy {
    /// Create a new MyPy type checker
    pub fn new() -> Self {
        Self {
            base: ToolBase {
                name: "mypy".to_string(),
                description: "Static type checker for Python".to_string(),
                tool_type: ToolType::TypeChecker,
                language: Language::Python,
            },
        }
    }

    /// Parse mypy output to extract issues
    fn parse_output(&self, output: &str) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Regex to match MyPy output format
        // Format: file:line: error: message
        let regex = Regex::new(r"(?m)^(.+):(\d+)(?::(\d+))?: (\w+): (.+)$").unwrap();

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
                .map(|m| m.as_str().parse::<usize>().unwrap_or(0));
            let level = capture.get(4).unwrap().as_str();
            let message = capture.get(5).unwrap().as_str();

            // Determine severity
            let severity = match level {
                "error" => IssueSeverity::Error,
                "note" => IssueSeverity::Info,
                _ => IssueSeverity::Warning,
            };

            issues.push(LintIssue {
                severity,
                message: message.to_string(),
                file: Some(file_path),
                line: Some(line),
                column,
                code: None,
                fix_available: false, // MyPy doesn't provide auto-fixes
            });
        }

        issues
    }

    /// Run mypy on multiple files to check for issues
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

        let mut command = Command::new("mypy");

        // Add common flags
        command.arg("--no-pretty");
        command.arg("--show-column-numbers");

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add all the files to check - explicitly pass each file path
        for file in &files_to_check {
            command.arg(file);
        }

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute mypy: {}", e),
        })?;

        // Parse the output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // MyPy can output to either stdout or stderr depending on the version
        let combined_output = format!("{}\n{}", stdout, stderr).trim().to_string();
        let issues = self.parse_output(&combined_output);

        Ok((issues, stdout, stderr))
    }
}

impl LintTool for MyPy {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn can_handle(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension() {
            ext == "py" || ext == "pyi"
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

        // Run mypy once for all files
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
        utils::is_command_available("mypy")
    }

    fn version(&self) -> Option<String> {
        utils::get_command_version("mypy", &["--version"])
    }
}
