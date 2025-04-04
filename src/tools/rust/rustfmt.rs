//! Rustfmt formatter for Rust

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crate::errors::ToolError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::{IssueSeverity, Language, LintIssue, LintResult, ToolInfo, ToolType};
use crate::tools::{LintTool, ToolBase};
use crate::utils;

/// Rustfmt formatter for Rust
pub struct Rustfmt {
    base: ToolBase,
}

impl Rustfmt {
    /// Create a new Rustfmt formatter
    pub fn new() -> Self {
        Self {
            base: ToolBase {
                name: "rustfmt".to_string(),
                description: "The Rust code formatter".to_string(),
                tool_type: ToolType::Formatter,
                languages: vec![Language::Rust],
            },
        }
    }

    /// Run rustfmt on a file to check if it needs formatting
    fn check_file(
        &self,
        file: &Path,
        config: &ModelsToolConfig,
    ) -> Result<(Vec<LintIssue>, String, String), ToolError> {
        // Always use the current directory as the project root
        // This assumes we're running from the project root
        let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Build cargo fmt command
        let mut command = Command::new("cargo");
        command.current_dir(&project_root);
        command.args(["fmt", "--", "--check"]);

        // Add custom config file if specified
        if let Some(config_file) = &config.executable_path {
            command.arg("--config-path").arg(config_file);
        }

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add the file to format - use the absolute path to avoid path resolution issues
        let abs_file_path = if file.is_absolute() {
            file.to_path_buf()
        } else {
            project_root.join(file)
        };

        command.arg(&abs_file_path);

        // Log the command
        utils::log_command(&command);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute cargo fmt: {}", e),
        })?;

        // Get stdout and stderr
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Parse the output
        if output.status.success() {
            // No formatting issues
            Ok((Vec::new(), stdout, stderr))
        } else {
            // File needs formatting
            Ok((
                vec![LintIssue {
                    severity: IssueSeverity::Style,
                    message: "File needs formatting".to_string(),
                    file: Some(file.to_path_buf()),
                    line: None,
                    column: None,
                    code: None,
                    fix_available: true,
                }],
                stdout,
                stderr,
            ))
        }
    }

    /// Fix formatting issues with rustfmt
    fn fix_file(
        &self,
        file: &Path,
        config: &ModelsToolConfig,
    ) -> Result<(String, String), ToolError> {
        // Always use the current directory as the project root
        // This assumes we're running from the project root
        let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Build cargo fmt command
        let mut command = Command::new("cargo");
        command.current_dir(&project_root);
        command.args(["fmt", "--"]);

        // Add custom config file if specified
        if let Some(config_file) = &config.executable_path {
            command.arg("--config-path").arg(config_file);
        }

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add the file to format - use the absolute path to avoid path resolution issues
        let abs_file_path = if file.is_absolute() {
            file.to_path_buf()
        } else {
            project_root.join(file)
        };

        command.arg(&abs_file_path);

        // Log the command
        utils::log_command(&command);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute cargo fmt: {}", e),
        })?;

        // Get stdout and stderr
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Check result
        if !output.status.success() {
            return Err(ToolError::ToolFailed {
                name: self.name().to_string(),
                code: output.status.code().unwrap_or(1),
                message: stderr.clone(),
            });
        }

        Ok((stdout, stderr))
    }
}

impl LintTool for Rustfmt {
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
                    languages: self.languages(),
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

        // Check if rustfmt is available
        if !self.is_available() {
            return Err(ToolError::NotFound(self.name().to_string()));
        }

        let start_time = Instant::now();
        let mut all_issues = Vec::new();
        let mut success = true;
        let mut all_stdout = String::new();
        let mut all_stderr = String::new();

        // Process each file
        for file in files {
            // Skip files we can't handle
            if !self.can_handle(file) {
                continue;
            }

            if config.check {
                // Check mode - just check if files need formatting
                match self.check_file(file, config) {
                    Ok((issues, stdout, stderr)) => {
                        if !issues.is_empty() {
                            success = false;
                            all_issues.extend(issues);
                        }
                        all_stdout.push_str(&stdout);
                        all_stderr.push_str(&stderr);
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            } else {
                // Format mode - actually format the files
                match self.fix_file(file, config) {
                    Ok((stdout, stderr)) => {
                        // Successfully formatted
                        all_stdout.push_str(&stdout);
                        all_stderr.push_str(&stderr);
                    }
                    Err(e) => {
                        success = false;
                        all_issues.push(LintIssue {
                            severity: IssueSeverity::Error,
                            message: format!("Failed to format file: {}", e),
                            file: Some(file.to_path_buf()),
                            line: None,
                            column: None,
                            code: None,
                            fix_available: false,
                        });
                    }
                }
            }
        }

        // Measure execution time
        let execution_time = start_time.elapsed();

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
            success,
            issues: all_issues,
            execution_time,
            stdout: if all_stdout.is_empty() {
                None
            } else {
                Some(all_stdout)
            },
            stderr: if all_stderr.is_empty() {
                None
            } else {
                Some(all_stderr)
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
        // Check if cargo fmt is available instead of standalone rustfmt
        utils::command_exists("cargo")
    }

    fn version(&self) -> Option<String> {
        // Run cargo fmt --version
        let output = Command::new("cargo")
            .args(["fmt", "--version"])
            .output()
            .ok()?;

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
