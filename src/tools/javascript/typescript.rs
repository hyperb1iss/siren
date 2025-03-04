//! TypeScript type checker

use regex::Regex;
use std::path::{Path, PathBuf};
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

    /// Parse TypeScript output to extract issues
    fn parse_output(&self, output: &str) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Regex to match TypeScript error output format
        // Format: file.ts(line,col): error TS2551: message
        let regex = Regex::new(r"(?m)^(.+)\((\d+),(\d+)\): (error|warning) (\w+): (.+)$").unwrap();

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
            let code = capture.get(5).unwrap().as_str();
            let message = capture.get(6).unwrap().as_str();

            // Determine severity
            let severity = match level {
                "error" => IssueSeverity::Error,
                _ => IssueSeverity::Warning,
            };

            issues.push(LintIssue {
                severity,
                message: message.to_string(),
                file: Some(file_path),
                line: Some(line),
                column,
                code: Some(code.to_string()),
                fix_available: false, // TypeScript doesn't provide auto-fixes through tsc
            });
        }

        issues
    }

    /// Run TypeScript on multiple files to check for issues
    fn check_files(
        &self,
        files: &[PathBuf],
        _config: &ModelsToolConfig,
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

        // TODO: Implement TypeScript execution
        // This should run tsc --noEmit on the files
        
        Ok((Vec::new(), String::new(), String::new()))
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
        utils::is_command_available("tsc")
    }

    fn version(&self) -> Option<String> {
        utils::get_command_version("tsc", &["--version"])
    }
} 