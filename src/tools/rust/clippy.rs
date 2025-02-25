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
        let regex =
            Regex::new(r"(?m)^\s*(?:warning|error)(?:\[(\w+)\])?: (.+?)\n\s+--> (.+?):(\d+):(\d+)")
                .unwrap();

        // Regex to match code snippets - looking for the code display section after an error/warning
        let code_regex = Regex::new(r"(?m)(\d+\s*\|\s*.*)\n").unwrap();

        for capture in regex.captures_iter(output) {
            let message = capture.get(2).unwrap().as_str().trim();
            let file_str = capture.get(3).unwrap().as_str();
            let line = capture
                .get(4)
                .unwrap()
                .as_str()
                .parse::<usize>()
                .unwrap_or(0);
            let column = capture
                .get(5)
                .unwrap()
                .as_str()
                .parse::<usize>()
                .unwrap_or(0);

            // Check if this is a warning or error
            let level = if output.contains("error") {
                "error"
            } else {
                "warning"
            };

            // Get the lint name if available
            let lint_name = capture.get(1).map(|m| m.as_str()).unwrap_or("");

            // Determine severity
            let severity = match level {
                "error" => IssueSeverity::Error,
                _ => IssueSeverity::Warning,
            };

            // Create a PathBuf for the file
            let file_path = PathBuf::from(file_str);

            let full_message = if !lint_name.is_empty() {
                format!("[{}] {}", lint_name, message)
            } else {
                message.to_string()
            };

            // Try to extract a code snippet for this issue
            let error_pos = capture.get(0).unwrap().start();
            let code_snippet = if let Some(code_match) = code_regex
                .captures_iter(&output[error_pos..])
                .next()
                .and_then(|c| c.get(1))
            {
                // Extract up to 3 lines of code for context
                let mut snippet = code_match.as_str().trim().to_string();
                let mut lines = 1;

                // Try to grab a couple more lines for context
                for cap in code_regex.captures_iter(&output[error_pos + code_match.end()..]) {
                    if let Some(m) = cap.get(1) {
                        snippet.push('\n');
                        snippet.push_str(m.as_str().trim());
                        lines += 1;
                        if lines >= 3 {
                            break;
                        }
                    }
                }

                Some(snippet)
            } else {
                None
            };

            issues.push(LintIssue {
                severity,
                message: full_message,
                file: Some(file_path),
                line: Some(line),
                column: Some(column),
                code: code_snippet.or_else(|| Some(lint_name.to_string())),
                fix_available: output.contains("help: "),
            });
        }

        if issues.is_empty() && output.contains("unused variable") {
            // Fallback for common case with unused variables
            let unused_regex =
                Regex::new(r"(?m)warning: unused variable: `([^`]+)`\s+--> ([^:]+):(\d+):(\d+)")
                    .unwrap();

            for capture in unused_regex.captures_iter(output) {
                let var_name = capture.get(1).unwrap().as_str();
                let file_str = capture.get(2).unwrap().as_str();
                let line = capture
                    .get(3)
                    .unwrap()
                    .as_str()
                    .parse::<usize>()
                    .unwrap_or(0);
                let column = capture
                    .get(4)
                    .unwrap()
                    .as_str()
                    .parse::<usize>()
                    .unwrap_or(0);

                issues.push(LintIssue {
                    severity: IssueSeverity::Warning,
                    message: format!("unused variable: `{}`", var_name),
                    file: Some(PathBuf::from(file_str)),
                    line: Some(line),
                    column: Some(column),
                    code: Some(format!("let {} = ...", var_name)),
                    fix_available: true,
                });
            }
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

        // Filter for Rust files only
        let rust_files: Vec<PathBuf> = files
            .iter()
            .filter(|file| self.can_handle(file))
            .cloned()
            .collect();

        // If no Rust files, return early with success
        if rust_files.is_empty() {
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
                stdout: Some("No Rust files to check".to_string()),
                stderr: None,
            });
        }

        // Get the project directory (parent of the first file)
        let project_dir = if let Some(file) = rust_files.first() {
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
        command.arg("--quiet"); // Suppress cargo output, only show clippy results

        // If specific files are specified, add them to the command
        // Clippy doesn't have a direct way to specify files, but we can at least
        // print which files we're checking
        if !rust_files.is_empty() && rust_files.len() < files.len() {
            // Log which files we're checking
            println!(
                "Running clippy on {} Rust files in {}",
                rust_files.len(),
                project_dir.display()
            );
        }

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Remove the JSON output format to get human-readable output
        // that's easier to parse with our regex
        // command.arg("--message-format=json");

        // Enable all clippy lints
        command.args(&["--", "-W", "clippy::all"]);

        // Set current directory to project directory
        command.current_dir(&project_dir);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute cargo clippy: {}", e),
        })?;

        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Debug output (only enable when working on this code)
        // println!("Clippy stdout: {}", stdout);
        // println!("Clippy stderr: {}", stderr);

        // Combine stdout and stderr for parsing
        let combined_output = format!("{}\n{}", stdout, stderr);

        // Parse issues
        let mut issues = self.parse_output(&combined_output, &project_dir);

        // Debug the issues found - only enable when needed
        // println!("Found {} issues from Clippy", issues.len());
        // for issue in &issues {
        //     println!("  - {:?}: {} at {:?}:{}:{}",
        //         issue.severity,
        //         issue.message,
        //         issue.file.as_ref().map(|f| f.display().to_string()).unwrap_or_default(),
        //         issue.line.unwrap_or(0),
        //         issue.column.unwrap_or(0)
        //     );
        // }

        // Filter issues to only include the files we were asked to check
        if !rust_files.is_empty() {
            let normalized_rust_files: Vec<PathBuf> = rust_files
                .iter()
                .map(|f| f.canonicalize().unwrap_or_else(|_| f.clone()))
                .collect();

            issues.retain(|issue| {
                if let Some(file_path) = &issue.file {
                    let normalized_path = file_path
                        .canonicalize()
                        .unwrap_or_else(|_| file_path.clone());
                    normalized_rust_files.iter().any(|f| *f == normalized_path)
                } else {
                    true // Keep issues without file info
                }
            });
        }

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
            stdout: Some(stdout),
            stderr: Some(stderr),
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
