//! DjLint formatter and linter for HTML/Templates

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::errors::ToolError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::{IssueSeverity, Language, LintIssue, LintResult, ToolInfo, ToolType};
use crate::tools::{LintTool, ToolBase};
use crate::utils;

/// DjLint linter for HTML/Templates
pub struct DjLint {
    base: ToolBase,
}

/// DjLint formatter for HTML/Templates
pub struct DjLintFormatter {
    base: ToolBase,
}

impl Default for DjLint {
    fn default() -> Self {
        Self::new()
    }
}

impl DjLint {
    /// Create a new DjLint linter
    pub fn new() -> Self {
        Self {
            base: ToolBase {
                name: "djlint".to_string(),
                description: "HTML template linter with support for Django, Jinja, Nunjucks, Handlebars, and more".to_string(),
                tool_type: ToolType::Linter,
                language: Language::Html,
            },
        }
    }

    /// Parse djlint output to extract issues
    pub fn parse_output(&self, stdout: &str, stderr: &str) -> Vec<LintIssue> {
        let mut issues = Vec::new();
        let mut current_file = None;

        // DjLint outputs issues in the format:
        // file.html
        // ───────────────────────────────────────────────────────────────────────────────
        // H021 14:8 Inline styles should be avoided. <div style="
        for line in stdout.lines().chain(stderr.lines()) {
            // Skip empty lines and separator lines
            if line.is_empty() || line.starts_with('─') {
                continue;
            }

            // Skip summary lines that appear at the end of output
            // Example: "4 files would be updated.: info: files, found 14 errors. [Linted]"
            if line.contains("files would be updated") || line.contains("[Linted]") {
                continue;
            }

            // If line doesn't start with an error code (H, T, etc), it might be a filename
            if !line.starts_with(|c: char| c.is_ascii_uppercase()) {
                current_file = Some(PathBuf::from(line.trim()));
                continue;
            }

            // Parse error line: "H021 14:8 Inline styles should be avoided. <div style="
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() >= 3 {
                let code = parts[0].to_string();

                // Parse line:column
                let pos_parts: Vec<&str> = parts[1].split(':').collect();
                let (line_num, column) = if pos_parts.len() == 2 {
                    (
                        pos_parts[0].parse::<usize>().ok(),
                        pos_parts[1].parse::<usize>().ok(),
                    )
                } else {
                    (None, None)
                };

                let message = parts[2].to_string();
                let severity = match code.chars().next() {
                    Some('H') => IssueSeverity::Warning, // HTML issues
                    Some('T') => IssueSeverity::Error,   // Template issues
                    _ => IssueSeverity::Info,
                };

                issues.push(LintIssue {
                    severity,
                    message,
                    file: current_file.clone(),
                    line: line_num,
                    column,
                    code: Some(code),
                    fix_available: true, // djlint can fix most issues
                });
            }
        }

        issues
    }

    /// Run djlint on files
    fn check_files(
        &self,
        files: &[PathBuf],
        config: &ModelsToolConfig,
    ) -> Result<(Vec<LintIssue>, String, String), ToolError> {
        let mut command = Command::new("djlint");

        // Add files to check
        for file in files {
            command.arg(file);
        }

        // Add both --check and --lint flags for proper linting
        command.arg("--check");
        command.arg("--lint");

        // Add any extra arguments from config
        for arg in &config.extra_args {
            command.arg(arg);
        }

        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute {}: {}", self.name(), e),
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let issues = self.parse_output(&stdout, &stderr);

        Ok((issues, stdout, stderr))
    }
}

impl Default for DjLintFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl DjLintFormatter {
    /// Create a new DjLint formatter
    pub fn new() -> Self {
        Self {
            base: ToolBase {
                name: "djlint-fmt".to_string(),
                description: "HTML template formatter with support for Django, Jinja, Nunjucks, Handlebars, and more".to_string(),
                tool_type: ToolType::Formatter,
                language: Language::Html,
            },
        }
    }

    /// Fix files using djlint
    fn fix_files(&self, files: &[PathBuf], config: &ModelsToolConfig) -> Result<(), ToolError> {
        let mut command = Command::new("djlint");

        // Add files to fix
        for file in files {
            command.arg(file);
        }

        // Add --reformat flag to fix issues
        command.arg("--reformat");

        // Add any extra arguments from config
        for arg in &config.extra_args {
            command.arg(arg);
        }

        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute {}: {}", self.name(), e),
        })?;

        // DjLint writes progress to stderr but it's not an error
        // Only treat as error if exit status indicates failure and there's a real error message
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            // Filter out progress messages
            let real_error = stderr
                .lines()
                .filter(|line| !line.contains("Reformatting") && !line.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n");

            if !real_error.is_empty() {
                return Err(ToolError::ExecutionFailed {
                    name: self.name().to_string(),
                    message: real_error,
                });
            }
        }

        Ok(())
    }

    /// Check if files need formatting
    fn check_formatting(
        &self,
        files: &[PathBuf],
        config: &ModelsToolConfig,
    ) -> Result<Vec<LintIssue>, ToolError> {
        let mut command = Command::new("djlint");

        // Add files to check
        for file in files {
            command.arg(file);
        }

        // Add --check flag to check formatting
        command.arg("--check");

        // Add any extra arguments from config
        for arg in &config.extra_args {
            command.arg(arg);
        }

        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute {}: {}", self.name(), e),
        })?;

        let mut issues = Vec::new();
        if !output.status.success() {
            // If check fails, it means files need formatting
            for file in files {
                issues.push(LintIssue {
                    severity: IssueSeverity::Warning,
                    message: "File needs formatting".to_string(),
                    file: Some(file.clone()),
                    line: None,
                    column: None,
                    code: Some("FMT001".to_string()),
                    fix_available: true,
                });
            }
        }

        Ok(issues)
    }
}

impl LintTool for DjLint {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn can_handle(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension() {
            matches!(
                ext.to_str(),
                Some("html") | Some("djhtml") | Some("jinja") | Some("j2") | Some("hbs")
            )
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
                execution_time: std::time::Duration::from_secs(0),
                stdout: None,
                stderr: None,
            });
        }

        // Check if djlint is available
        if !self.is_available() {
            return Err(ToolError::NotFound(self.name().to_string()));
        }

        let start_time = Instant::now();

        // Filter for HTML files only
        let html_files: Vec<PathBuf> = files
            .iter()
            .filter(|file| self.can_handle(file))
            .cloned()
            .collect();

        // If no HTML files, return early with success
        if html_files.is_empty() {
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
                execution_time: start_time.elapsed(),
                stdout: None,
                stderr: None,
            });
        }

        // Run djlint check on files
        let (issues, stdout, stderr) = self.check_files(&html_files, config)?;

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
            execution_time: start_time.elapsed(),
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
        utils::is_command_available("djlint")
    }

    fn version(&self) -> Option<String> {
        utils::get_command_version("djlint", &["--version"])
    }
}

impl LintTool for DjLintFormatter {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn can_handle(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension() {
            matches!(
                ext.to_str(),
                Some("html") | Some("djhtml") | Some("jinja") | Some("j2") | Some("hbs")
            )
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
                execution_time: std::time::Duration::from_secs(0),
                stdout: None,
                stderr: None,
            });
        }

        // Check if djlint is available
        if !self.is_available() {
            return Err(ToolError::NotFound(self.name().to_string()));
        }

        let start_time = Instant::now();

        // Filter for HTML files only
        let html_files: Vec<PathBuf> = files
            .iter()
            .filter(|file| self.can_handle(file))
            .cloned()
            .collect();

        // If no HTML files, return early with success
        if html_files.is_empty() {
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
                execution_time: start_time.elapsed(),
                stdout: None,
                stderr: None,
            });
        }

        // Check if we should fix issues
        if config.auto_fix {
            self.fix_files(&html_files, config)?;

            // Return success result after fixing
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
                execution_time: start_time.elapsed(),
                stdout: None,
                stderr: None,
            });
        }

        // Check formatting
        let issues = self.check_formatting(&html_files, config)?;

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
            execution_time: start_time.elapsed(),
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
        utils::is_command_available("djlint")
    }

    fn version(&self) -> Option<String> {
        utils::get_command_version("djlint", &["--version"])
    }
}
