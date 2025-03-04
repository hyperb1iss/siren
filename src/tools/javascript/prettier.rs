//! Prettier formatter for JavaScript and TypeScript

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::errors::ToolError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::{Language, LintIssue, LintResult, ToolInfo, ToolType};
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
                description: "Opinionated code formatter for JavaScript, TypeScript, and more".to_string(),
                tool_type: ToolType::Formatter,
                language: Language::JavaScript, // Also handles TypeScript
            },
        }
    }

    /// Check if a file can be formatted with Prettier
    fn can_format_file(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            matches!(
                ext,
                "js" | "jsx" | "ts" | "tsx" | "json" | "css" | "scss" | "less" | "html" | "vue" | "graphql" | "md" | "yaml" | "yml"
            )
        } else {
            false
        }
    }

    /// Check a file for formatting issues
    fn check_file(
        &self,
        _file_path: &Path,
        _config: &ModelsToolConfig,
    ) -> Result<Vec<LintIssue>, ToolError> {
        // TODO: Implement check_file logic
        // This should run prettier --check on the file
        Ok(Vec::new())
    }

    /// Fix formatting issues in a file
    fn fix_file(
        &self,
        _file_path: &Path,
        _config: &ModelsToolConfig,
    ) -> Result<(), ToolError> {
        // TODO: Implement fix_file logic
        // This should run prettier --write on the file
        Ok(())
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

        // Check if we should fix issues
        let fix_mode = config.auto_fix;

        // Process each file
        for file in files {
            if !self.can_handle(file) {
                continue;
            }

            if fix_mode {
                // Fix the file
                self.fix_file(file, config)?
            } else {
                // Check the file
                match self.check_file(file, config) {
                    Ok(file_issues) => {
                        issues.extend(file_issues);
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }

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

    fn language(&self) -> Language {
        self.base.language
    }

    fn description(&self) -> &str {
        &self.base.description
    }

    fn is_available(&self) -> bool {
        utils::is_command_available("prettier")
    }

    fn version(&self) -> Option<String> {
        utils::get_command_version("prettier", &["--version"])
    }
} 