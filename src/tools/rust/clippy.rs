//! Clippy linter for Rust

use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crate::errors::ToolError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::{IssueSeverity, Language, LintIssue, LintResult, ToolInfo, ToolType};
use crate::tools::{LintTool, ToolBase};
use crate::utils;

/// Clippy linter for Rust
pub struct Clippy {
    base: ToolBase,
}

impl Clippy {
    /// Create a new Clippy linter
    pub fn new() -> Self {
        Self {
            base: ToolBase {
                name: "clippy".to_string(),
                description:
                    "A collection of lints to catch common mistakes and improve your Rust code"
                        .to_string(),
                tool_type: ToolType::Linter,
                language: Language::Rust,
                priority: 5,
            },
        }
    }

    /// Parse clippy output to extract issues
    fn parse_output(&self, output: &str, _dir: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Regex to match Clippy warnings and errors
        // Format: file:line:column: error/warning: message
        let regex = Regex::new(r"(?m)^(.+):(\d+):(\d+):\s+(\w+):\s+(.+)$").unwrap();

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
            let level = capture.get(4).unwrap().as_str();
            let message = capture.get(5).unwrap().as_str();

            // Determine severity
            let severity = match level {
                "error" => IssueSeverity::Error,
                "warning" => IssueSeverity::Warning,
                "note" => IssueSeverity::Info,
                "help" => IssueSeverity::Info,
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
                code: None,
                fix_available: message.contains("help: "),
            });
        }

        issues
    }
}

impl LintTool for Clippy {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn can_handle(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension() {
            ext == "rs"
        } else {
            false
        }
    }

    fn execute(
        &self,
        files: &[PathBuf],
        config: &ModelsToolConfig,
    ) -> Result<LintResult, ToolError> {
        // Skip if not enabled
        if !config.enabled {
            return Ok(LintResult {
                tool_name: self.name().to_string(),
                tool: Some(ToolInfo {
                    name: self.name().to_string(),
                    tool_type: self.tool_type(),
                    language: self.language(),
                    available: self.is_available(),
                    version: self.version(),
                    description: self.description().to_string(),
                }),
                success: true,
                issues: Vec::new(),
                execution_time: Duration::from_secs(0),
                stdout: None,
                stderr: None,
            });
        }

        // Check if clippy is available
        if !self.is_available() {
            return Err(ToolError::NotFound(self.name().to_string()));
        }

        // We need to run clippy in the context of a Cargo project,
        // so we'll just run it on the whole project instead of specific files

        let start_time = Instant::now();

        // Get the project directory (parent of the first file)
        let project_dir = if let Some(file) = files.first() {
            let mut dir = file.parent().unwrap_or(Path::new(".")).to_path_buf();
            // Find the Cargo.toml file
            while !dir.join("Cargo.toml").exists() {
                if let Some(parent) = dir.parent() {
                    dir = parent.to_path_buf();
                } else {
                    break;
                }
            }
            dir
        } else {
            // No files specified, use current directory
            PathBuf::from(".")
        };

        // Run clippy
        let mut command = Command::new("cargo");
        command.arg("clippy");

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Use JSON output format for better parsing
        command.arg("--message-format=json");

        // Set current directory to project directory
        command.current_dir(&project_dir);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute cargo clippy: {}", e),
        })?;

        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Combine stdout and stderr for parsing
        let combined_output = format!("{}\n{}", stdout, stderr);

        // Parse issues
        let issues = self.parse_output(&combined_output, &project_dir);

        // Measure execution time
        let execution_time = start_time.elapsed();

        // Determine success
        let success =
            output.status.success() && !issues.iter().any(|i| i.severity == IssueSeverity::Error);

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
            success,
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
        // Check for both cargo and clippy
        utils::command_exists("cargo") && 
        // Try running cargo clippy -V to see if clippy is installed
        Command::new("cargo").args(["clippy", "-V"]).output().map(|o| o.status.success()).unwrap_or(false)
    }

    fn version(&self) -> Option<String> {
        // Run cargo clippy -V
        let output = Command::new("cargo").args(["clippy", "-V"]).output().ok()?;

        if output.status.success() {
            // Parse the version from output
            let version = String::from_utf8_lossy(&output.stdout).to_string();
            let version = version.trim();
            Some(version.to_string())
        } else {
            None
        }
    }
}
