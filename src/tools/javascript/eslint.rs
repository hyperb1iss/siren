//! ESLint linter for JavaScript and TypeScript

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::errors::ToolError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::{IssueSeverity, Language, LintIssue, LintResult, ToolInfo, ToolType};
use crate::tools::{LintTool, ToolBase};
use crate::utils;

/// ESLint linter for JavaScript and TypeScript
pub struct ESLint {
    base: ToolBase,
}

impl Default for ESLint {
    fn default() -> Self {
        Self::new()
    }
}

impl ESLint {
    /// Create a new ESLint linter
    pub fn new() -> Self {
        Self {
            base: ToolBase {
                name: "eslint".to_string(),
                description: "Pluggable linting utility for JavaScript and TypeScript".to_string(),
                tool_type: ToolType::Linter,
                language: Language::TypeScript, // Change to TypeScript since it handles both
            },
        }
    }

    /// Run ESLint on multiple files to check for issues
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

        let mut command = Command::new("npx");
        command.args(["eslint", "--format=json"]);

        // Add config file if specified
        if let Some(config_file) = &config.executable_path {
            command.args(["--config", config_file]);
        }

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add all files to check
        for file in files_to_check {
            command.arg(file);
        }

        // Log the command
        utils::log_command(&command);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute eslint: {}", e),
        })?;

        // Get stdout and stderr
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Parse JSON output to get issues
        let mut issues = Vec::new();
        if !stdout.is_empty() {
            if let Ok(json_output) = serde_json::from_str::<serde_json::Value>(&stdout) {
                if let Some(files) = json_output.as_array() {
                    for file in files {
                        if let Some(messages) = file.get("messages").and_then(|m| m.as_array()) {
                            for message in messages {
                                let severity =
                                    match message.get("severity").and_then(|s| s.as_u64()) {
                                        Some(2) => IssueSeverity::Error,
                                        Some(1) => IssueSeverity::Warning,
                                        _ => IssueSeverity::Info,
                                    };

                                let message_text = message
                                    .get("message")
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("Unknown issue")
                                    .to_string();

                                let file_path = file
                                    .get("filePath")
                                    .and_then(|f| f.as_str())
                                    .map(PathBuf::from);

                                let line = message
                                    .get("line")
                                    .and_then(|l| l.as_u64())
                                    .map(|l| l as usize);

                                let column = message
                                    .get("column")
                                    .and_then(|c| c.as_u64())
                                    .map(|c| c as usize);

                                let rule_id = message.get("ruleId").and_then(|r| r.as_str());

                                issues.push(LintIssue {
                                    severity,
                                    message: message_text,
                                    file: file_path,
                                    line,
                                    column,
                                    code: rule_id.map(String::from),
                                    fix_available: message.get("fix").is_some(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok((issues, stdout, stderr))
    }

    /// Fix issues in multiple files
    fn fix_files(&self, files: &[PathBuf], _config: &ModelsToolConfig) -> Result<(), ToolError> {
        // Skip if no files can be handled
        let files_to_fix: Vec<&Path> = files
            .iter()
            .filter(|file| self.can_handle(file))
            .map(|file| file.as_path())
            .collect();

        if files_to_fix.is_empty() {
            return Ok(());
        }

        // TODO: Implement ESLint fix execution
        // This should run eslint --fix on the files

        Ok(())
    }
}

impl LintTool for ESLint {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn can_handle(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            matches!(ext, "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx")
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
        if config.auto_fix {
            // Fix the files
            self.fix_files(files, config)?;

            // Return an empty result since we fixed the issues
            let execution_time = start.elapsed();
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
                execution_time,
                stdout: None,
                stderr: None,
            });
        }

        // Run ESLint once for all files
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
        Language::TypeScript // Change to TypeScript since it handles both
    }

    fn description(&self) -> &str {
        &self.base.description
    }

    fn is_available(&self) -> bool {
        // Check if npx is available first
        if !utils::command_exists("npx") {
            return false;
        }

        // Try running eslint --version through npx
        let mut command = Command::new("npx");
        command.args(["eslint", "--version"]);

        // Log the command
        utils::log_command(&command);

        command
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn version(&self) -> Option<String> {
        // Run eslint --version through npx
        let mut command = Command::new("npx");
        command.args(["eslint", "--version"]);

        // Log the command
        utils::log_command(&command);

        let output = command.output().ok()?;

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
