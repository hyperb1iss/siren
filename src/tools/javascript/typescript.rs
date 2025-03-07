//! TypeScript type checker

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::errors::ToolError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::{IssueSeverity, Language, LintIssue, LintResult, ToolInfo, ToolType};
use crate::tools::{LintTool, ToolBase};
use crate::utils;

/// TypeScript type checker
pub struct TypeScript {
    base: ToolBase,
}

impl Default for TypeScript {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeScript {
    /// Create a new TypeScript type checker
    pub fn new() -> Self {
        Self {
            base: ToolBase {
                name: "typescript".to_string(),
                description: "TypeScript type checker".to_string(),
                tool_type: ToolType::TypeChecker,
                language: Language::TypeScript,
            },
        }
    }

    /// Run TypeScript type checker on multiple files
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
        command.args([
            "tsc",
            "--noEmit",
            "--jsx",
            "react-jsx",         // Enable JSX support
            "--esModuleInterop", // Better module compatibility
            "--skipLibCheck",    // Skip type checking of node_modules
            "--noImplicitAny",   // Better type safety
        ]);

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
            message: format!("Failed to execute tsc: {}", e),
        })?;

        // Get stdout and stderr
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Parse TypeScript error output
        let mut issues = Vec::new();
        for line in stderr.lines() {
            // TypeScript error format: file(line,col): error TS2345: message
            if let Some(captures) =
                regex::Regex::new(r"^(.+?)\((\d+),(\d+)\): (error|warning) TS(\d+): (.+)$")
                    .ok()
                    .and_then(|re| re.captures(line))
            {
                let file_path = PathBuf::from(captures.get(1).unwrap().as_str());
                let line_num = captures
                    .get(2)
                    .unwrap()
                    .as_str()
                    .parse::<usize>()
                    .unwrap_or(0);
                let column = captures
                    .get(3)
                    .unwrap()
                    .as_str()
                    .parse::<usize>()
                    .unwrap_or(0);
                let severity_str = captures.get(4).unwrap().as_str();
                let code = captures.get(5).map(|m| format!("TS{}", m.as_str()));
                let message = captures.get(6).unwrap().as_str().to_string();

                let severity = match severity_str {
                    "error" => IssueSeverity::Error,
                    "warning" => IssueSeverity::Warning,
                    _ => IssueSeverity::Info,
                };

                issues.push(LintIssue {
                    severity,
                    message,
                    file: Some(file_path),
                    line: Some(line_num),
                    column: Some(column),
                    code,
                    fix_available: false,
                });
            }
        }

        Ok((issues, stdout, stderr))
    }
}

impl LintTool for TypeScript {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn can_handle(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            matches!(ext, "ts" | "tsx")
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

        // Run TypeScript once for all files
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
        // Check if npx is available first
        if !utils::command_exists("npx") {
            return false;
        }

        // Try running tsc --version through npx
        let mut command = Command::new("npx");
        command.args(["tsc", "--version"]);

        // Log the command
        utils::log_command(&command);

        command
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn version(&self) -> Option<String> {
        // Run tsc --version through npx
        let mut command = Command::new("npx");
        command.args(["tsc", "--version"]);

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
