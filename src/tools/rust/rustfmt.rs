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
                language: Language::Rust,
                priority: 10,
            },
        }
    }

    /// Run rustfmt on a file to check if it needs formatting
    fn check_file(
        &self,
        file: &Path,
        config: &ModelsToolConfig,
    ) -> Result<Vec<LintIssue>, ToolError> {
        let mut command = Command::new("rustfmt");
        command.arg("--check");

        // Add custom config file if specified
        if let Some(config_file) = &config.executable_path {
            command.arg("--config-path").arg(config_file);
        }

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add the file to format
        command.arg(file);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute rustfmt: {}", e),
        })?;

        // Parse the output
        if output.status.success() {
            // No formatting issues
            Ok(Vec::new())
        } else {
            // File needs formatting
            Ok(vec![LintIssue {
                severity: IssueSeverity::Style,
                message: "File needs formatting".to_string(),
                file: Some(file.to_path_buf()),
                line: None,
                column: None,
                code: None,
                fix_available: true,
            }])
        }
    }

    /// Fix formatting issues with rustfmt
    fn fix_file(&self, file: &Path, config: &ModelsToolConfig) -> Result<(), ToolError> {
        let mut command = Command::new("rustfmt");

        // Add custom config file if specified
        if let Some(config_file) = &config.executable_path {
            command.arg("--config-path").arg(config_file);
        }

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add the file to format
        command.arg(file);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute rustfmt: {}", e),
        })?;

        // Check result
        if !output.status.success() {
            return Err(ToolError::ToolFailed {
                name: self.name().to_string(),
                code: output.status.code().unwrap_or(1),
                message: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(())
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

        // Check if rustfmt is available
        if !self.is_available() {
            return Err(ToolError::NotFound(self.name().to_string()));
        }

        let start_time = Instant::now();
        let mut all_issues = Vec::new();
        let mut success = true;

        // Process each file
        for file in files {
            // Skip files we can't handle
            if !self.can_handle(file) {
                continue;
            }

            // Check if file needs formatting
            match self.check_file(file, config) {
                Ok(issues) => {
                    if !issues.is_empty() {
                        success = false;
                        all_issues.extend(issues);
                    }
                }
                Err(e) => {
                    return Err(e);
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
                language: self.language(),
                available: self.is_available(),
                version: self.version(),
                description: self.description().to_string(),
            }),
            success,
            issues: all_issues,
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
        utils::command_exists("rustfmt")
    }

    fn version(&self) -> Option<String> {
        // Run rustfmt --version
        let output = Command::new("rustfmt").arg("--version").output().ok()?;

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
