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
                languages: vec![Language::JavaScript, Language::TypeScript],
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
        let files_to_check: Vec<PathBuf> = files
            .iter()
            .filter(|file| self.can_handle(file))
            .cloned()
            .collect();

        if files_to_check.is_empty() {
            return Ok((Vec::new(), String::new(), String::new()));
        }

        // Use the more generic path optimizer that collapses to top-level directories
        let optimized_paths = utils::optimize_paths_for_tools(&files_to_check);

        // Filter out problematic directories that might contain build artifacts
        let filtered_paths: Vec<PathBuf> = optimized_paths
            .into_iter()
            .filter(|path| {
                // Skip directories that are likely to contain build artifacts or node_modules
                if path.is_dir() {
                    let path_str = path.to_string_lossy();
                    !path_str.contains("/.next/")
                        && !path_str.contains("/node_modules/")
                        && !path_str.contains("/dist/")
                        && !path_str.contains("/build/")
                } else {
                    true
                }
            })
            .collect();

        if filtered_paths.is_empty() {
            return Ok((Vec::new(), String::new(), String::new()));
        }

        let mut command = Command::new("npx");
        command.args(["eslint", "--format=json"]);

        // Add config file if specified
        if let Some(config_file) = &config.executable_path {
            command.args(["--config", config_file]);
        }

        // Add ignore patterns for build artifacts
        command.args(["--ignore-pattern", "**/.next/**"]);
        command.args(["--ignore-pattern", "**/node_modules/**"]);
        command.args(["--ignore-pattern", "**/dist/**"]);
        command.args(["--ignore-pattern", "**/build/**"]);

        // Add extra arguments
        for arg in &config.extra_args {
            command.arg(arg);
        }

        // Add all valid paths to check
        for path in &filtered_paths {
            command.arg(path);
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
        let files_to_fix: Vec<PathBuf> = files
            .iter()
            .filter(|file| self.can_handle(file))
            .cloned()
            .collect();

        if files_to_fix.is_empty() {
            return Ok(());
        }

        // Use the more generic path optimizer that collapses to top-level directories
        let optimized_paths = utils::optimize_paths_for_tools(&files_to_fix);

        // Filter out problematic directories that might contain build artifacts
        let filtered_paths: Vec<PathBuf> = optimized_paths
            .into_iter()
            .filter(|path| {
                // Skip directories that are likely to contain build artifacts or node_modules
                if path.is_dir() {
                    let path_str = path.to_string_lossy();
                    !path_str.contains("/.next/")
                        && !path_str.contains("/node_modules/")
                        && !path_str.contains("/dist/")
                        && !path_str.contains("/build/")
                } else {
                    true
                }
            })
            .collect();

        if filtered_paths.is_empty() {
            return Ok(());
        }

        let mut command = Command::new("npx");
        command.args(["eslint", "--fix"]);

        // Add ignore patterns for build artifacts
        command.args(["--ignore-pattern", "**/.next/**"]);
        command.args(["--ignore-pattern", "**/node_modules/**"]);
        command.args(["--ignore-pattern", "**/dist/**"]);
        command.args(["--ignore-pattern", "**/build/**"]);

        // Add all valid paths to fix
        for path in &filtered_paths {
            command.arg(path);
        }

        // Log the command
        utils::log_command(&command);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute eslint --fix: {}", e),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(ToolError::ExecutionFailed {
                name: self.name().to_string(),
                message: format!("ESLint fix failed: {}", stderr),
            });
        }

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
                    languages: self.languages(),
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
