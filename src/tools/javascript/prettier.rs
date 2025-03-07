//! Prettier formatter for JavaScript and TypeScript

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::errors::ToolError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::{IssueSeverity, Language, LintIssue, LintResult, ToolInfo, ToolType};
use crate::tools::{LintTool, ToolBase};
use crate::utils;

/// Prettier formatter for JavaScript and TypeScript
pub struct Prettier {
    base: ToolBase,
}

impl Default for Prettier {
    fn default() -> Self {
        Self::new()
    }
}

impl Prettier {
    /// Create a new Prettier formatter
    pub fn new() -> Self {
        Self {
            base: ToolBase {
                name: "prettier".to_string(),
                description: "Opinionated code formatter for JavaScript, TypeScript, and more"
                    .to_string(),
                tool_type: ToolType::Formatter,
                languages: vec![Language::JavaScript, Language::TypeScript],
            },
        }
    }

    /// Check if a file can be formatted with Prettier
    fn can_format_file(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            matches!(
                ext,
                "js" | "jsx"
                    | "ts"
                    | "tsx"
                    | "json"
                    | "css"
                    | "scss"
                    | "less"
                    | "html"
                    | "vue"
                    | "graphql"
                    | "md"
                    | "yaml"
                    | "yml"
            )
        } else {
            false
        }
    }
}

impl LintTool for Prettier {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn can_handle(&self, file_path: &Path) -> bool {
        self.can_format_file(file_path)
    }

    fn execute(
        &self,
        files: &[PathBuf],
        config: &ModelsToolConfig,
    ) -> Result<LintResult, ToolError> {
        let start = Instant::now();
        let mut issues = Vec::new();

        // Filter files to only those we can handle
        let files_to_process: Vec<PathBuf> = files
            .iter()
            .filter(|file| self.can_handle(file))
            .cloned()
            .collect();

        if files_to_process.is_empty() {
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
                execution_time: start.elapsed(),
                stdout: None,
                stderr: None,
            });
        }

        // Optimize paths by grouping files with supported extensions
        let supported_exts = &[
            "js", "jsx", "ts", "tsx", "json", "css", "scss", "less", "html", "vue", "graphql",
            "md", "yaml", "yml",
        ];

        // Use the unified path finder to get optimized and gitignore-respecting paths
        // Prettier can handle directories directly, so use_directories=true
        let valid_paths = utils::find_tool_paths(
            &files_to_process,
            supported_exts,
            |path| self.can_handle(path),
            true,
        );

        // Check if we should fix issues
        let fix_mode = config.auto_fix;

        if fix_mode {
            // In fix mode, we can run prettier on all paths at once
            let mut command = Command::new("npx");
            command.args(["prettier", "--write"]);

            // Add extra arguments
            for arg in &config.extra_args {
                command.arg(arg);
            }

            // Add all valid paths
            for path in &valid_paths {
                command.arg(path);
            }

            // Log the command
            utils::log_command(&command);

            // Run the command
            let output = command.output().map_err(|e| ToolError::ExecutionFailed {
                name: self.name().to_string(),
                message: format!("Failed to execute prettier: {}", e),
            })?;

            // Check if the command succeeded
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                return Err(ToolError::ToolFailed {
                    name: self.name().to_string(),
                    code: output.status.code().unwrap_or(1),
                    message: stderr,
                });
            }
        } else {
            // In check mode, we run prettier on all paths at once
            let mut command = Command::new("npx");
            command.args(["prettier", "--check"]);

            // Add extra arguments
            for arg in &config.extra_args {
                command.arg(arg);
            }

            // Add all valid paths
            for path in &valid_paths {
                command.arg(path);
            }

            // Log the command
            utils::log_command(&command);

            // Run the command
            let output = command.output().map_err(|e| ToolError::ExecutionFailed {
                name: self.name().to_string(),
                message: format!("Failed to execute prettier: {}", e),
            })?;

            // If the command failed, it means formatting issues were found
            if !output.status.success() {
                // For each file that needs formatting, add an issue
                for file in &files_to_process {
                    issues.push(LintIssue {
                        severity: IssueSeverity::Style,
                        message: "File needs formatting".to_string(),
                        file: Some(file.clone()),
                        line: None,
                        column: None,
                        code: None,
                        fix_available: true,
                    });
                }
            }
        }

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

        // Try running prettier --version through npx
        let mut command = Command::new("npx");
        command.args(["prettier", "--version"]);

        // Log the command
        utils::log_command(&command);

        command
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn version(&self) -> Option<String> {
        // Run prettier --version through npx
        let mut command = Command::new("npx");
        command.args(["prettier", "--version"]);

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
