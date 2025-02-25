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

/// Clippy automatic fixer for Rust
pub struct ClippyFixer {
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

    /// Find the directory containing Cargo.toml by walking up the directory tree
    fn find_cargo_toml_dir(&self, file_path: &Path) -> Result<PathBuf, ToolError> {
        let file_dir = if file_path.is_file() {
            file_path.parent().unwrap_or(Path::new("."))
        } else {
            file_path
        };

        // Convert to absolute path if it's not already
        let file_dir = if file_dir.is_absolute() {
            file_dir.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(file_dir)
        };

        println!("Starting directory search from: {}", file_dir.display());

        let mut current_dir = Some(file_dir);

        while let Some(dir) = current_dir {
            let cargo_toml = dir.join("Cargo.toml");
            println!("Checking for Cargo.toml at: {}", cargo_toml.display());
            if cargo_toml.exists() {
                println!("Found Cargo.toml at: {}", dir.display());
                return Ok(dir);
            }

            // Move up to parent directory
            current_dir = dir.parent().map(|p| p.to_path_buf());
        }

        // If we can't find a Cargo.toml, use the current working directory
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        println!(
            "No Cargo.toml found, using current directory: {}",
            cwd.display()
        );
        Ok(cwd)
    }

    /// Run clippy with the given arguments
    fn run_clippy(
        &self,
        rust_files: &[PathBuf],
        project_dir: &Path,
        _config: &ModelsToolConfig,
        fix_mode: bool,
    ) -> Result<(LintResult, String, String), ToolError> {
        let start_time = Instant::now();

        // Build the clippy command with minimal arguments
        let mut command = Command::new("cargo");
        command.arg("clippy");

        // If fixing, add the fix flag
        if fix_mode {
            command.arg("--fix");
        }

        // Add -- separator for clippy-specific arguments
        command.arg("--");

        // Add a basic warning level
        command.arg("-W");
        command.arg("clippy::all");

        // Set current directory to project_dir
        command.current_dir(project_dir);

        // Show the full command being executed
        let cmd_str = format!(
            "cargo clippy -- -W clippy::all in {}",
            project_dir.display()
        );
        println!("Executing: {}", cmd_str);

        // Run the command
        let output = command.output().map_err(|e| ToolError::ExecutionFailed {
            name: self.name().to_string(),
            message: format!("Failed to execute clippy: {}", e),
        })?;

        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Always display output status
        println!("Clippy completed with status: {}", output.status);

        // Display the output directly
        if !stdout.is_empty() {
            println!("STDOUT:\n{}", stdout);
        }
        if !stderr.is_empty() {
            println!("STDERR:\n{}", stderr);
        }

        // Combine stdout and stderr for parsing
        let combined_output = format!("{}\n{}", stdout, stderr);

        // Parse issues
        let mut issues = self.parse_output(&combined_output, project_dir);

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

        let result = LintResult {
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
            stdout: Some(stdout.clone()),
            stderr: Some(stderr.clone()),
        };

        Ok((result, stdout, stderr))
    }
}

impl ClippyFixer {
    /// Create a new ClippyFixer
    pub fn new() -> Self {
        Self {
            base: ToolBase {
                name: "clippy-fix".to_string(),
                description: "Automatically fix common mistakes in your Rust code with Clippy"
                    .to_string(),
                tool_type: ToolType::Fixer,
                language: Language::Rust,
            },
        }
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
        let project_dir = self.find_cargo_toml_dir(rust_files.first().unwrap())?;

        // Run clippy in check mode (no fixing)
        let (result, _, _) = self.run_clippy(&rust_files, &project_dir, config, false)?;
        Ok(result)
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

impl LintTool for ClippyFixer {
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
                stdout: Some("No Rust files to fix".to_string()),
                stderr: None,
            });
        }

        // Create a Clippy instance to use its functionality
        let clippy = Clippy::new();

        // Get the project directory (parent of the first file)
        let project_dir = clippy.find_cargo_toml_dir(rust_files.first().unwrap())?;

        // Run clippy in fix mode
        let (mut result, _stdout, stderr) =
            clippy.run_clippy(&rust_files, &project_dir, config, true)?;

        // Update the tool name and type for our result
        result.tool_name = self.name().to_string();
        if let Some(tool_info) = &mut result.tool {
            tool_info.name = self.name().to_string();
            tool_info.tool_type = self.tool_type();
            tool_info.description = self.description().to_string();
        }

        // If stderr contains "fixed", extract the count of fixed issues
        if let Some(fixed_count) = stderr.lines().find_map(|line| {
            if line.contains("fixed ") && line.contains(" warning") {
                line.split_whitespace()
                    .find(|word| word.parse::<usize>().is_ok())
                    .and_then(|num| num.parse::<usize>().ok())
            } else {
                None
            }
        }) {
            // Add a fix message at the top of the result
            if fixed_count > 0 {
                result.issues.push(LintIssue {
                    severity: IssueSeverity::Info,
                    message: format!("✅ Fixed {} issues automatically", fixed_count),
                    file: None,
                    line: None,
                    column: None,
                    code: None,
                    fix_available: false,
                });
            }
        }

        Ok(result)
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
        // Use the same availability check as Clippy
        let clippy = Clippy::new();
        clippy.is_available()
    }

    fn version(&self) -> Option<String> {
        // Use the same version check as Clippy
        let clippy = Clippy::new();
        clippy.version()
    }
}
