use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::cli::{CheckArgs, Verbosity};
use crate::config::SirenConfig;
use crate::detection::ProjectDetector;
use crate::errors::SirenError;
use crate::models::tools::ToolConfig;
use crate::models::{ProjectInfo, ToolType};
use crate::output::terminal;
use crate::output::OutputFormatter;
use crate::runner::ToolRunner;
use crate::tools::{LintTool, ToolRegistry};
use colored::*;
use log::{debug, warn};

/// Command handler for the check (lint) command
pub struct CheckCommand<D, R, O>
where
    D: ProjectDetector,
    R: ToolRegistry + Clone,
    O: OutputFormatter,
{
    detector: D,
    tool_registry: R,
    output_formatter: O,
    verbosity: Verbosity,
}

impl<D, R, O> CheckCommand<D, R, O>
where
    D: ProjectDetector,
    R: ToolRegistry + Clone,
    O: OutputFormatter,
{
    /// Create a new check command handler
    pub fn new(detector: D, tool_registry: R, output_formatter: O, verbosity: Verbosity) -> Self {
        Self {
            detector,
            tool_registry,
            output_formatter,
            verbosity,
        }
    }

    /// Execute the check command
    pub async fn execute(
        &self,
        args: CheckArgs,
        paths: Vec<PathBuf>,
        git_modified_only: bool,
        config: &SirenConfig,
    ) -> Result<(), SirenError> {
        // Clone paths from args to avoid ownership issues
        let args_paths = args.paths.clone();

        // Combine paths from the Cli struct and CheckArgs
        let all_paths = if args_paths.is_empty() {
            paths.clone()
        } else {
            args_paths
        };

        // Get project root directory
        let dir = all_paths
            .first()
            .map(|p| p.as_path())
            .unwrap_or_else(|| Path::new("."));

        // Extract path patterns (anything after the first path)
        let patterns: Vec<String> = if all_paths.len() > 1 {
            all_paths
                .iter()
                .skip(1)
                .map(|p| p.to_string_lossy().to_string())
                .collect()
        } else {
            Vec::new()
        };

        // Detect project information with patterns if provided
        let project_info = if !patterns.is_empty() {
            self.detect_project_with_patterns(dir, &patterns)?
        } else {
            self.detect_project(&all_paths)?
        };

        // Display detected project info based on verbosity
        if self.verbosity >= Verbosity::Normal {
            let info_output = self.output_formatter.format_detection(&project_info);
            println!("{}", info_output);
        }

        // Select appropriate tools based on project_info and args
        let tools = self.select_tools_for_check(&project_info, &args, config)?;

        // Count files by language for information purposes
        let _total_files: usize = project_info.file_counts.values().sum();
        let _files_by_language = project_info.file_counts.clone();

        // Get files to check
        let files = if git_modified_only {
            let dir = all_paths
                .first()
                .map(|p| p.as_path())
                .unwrap_or_else(|| Path::new("."));
            crate::utils::get_git_modified_files(dir)?
        } else if all_paths.is_empty() {
            // If no paths are specified, use the current directory
            let current_dir = Path::new(".");
            crate::utils::collect_files_with_gitignore(current_dir)?
        } else {
            // Expand directories to files
            let mut all_files = Vec::new();

            for path in &all_paths {
                if path.is_dir() {
                    let dir_files = crate::utils::collect_files_with_gitignore(path)?;
                    all_files.extend(dir_files);
                } else if path.is_file() {
                    all_files.push(path.clone());
                }
            }

            all_files
        };

        // Get default tool config
        let default_tool_config = config.tools.get("default").cloned().unwrap_or_default();

        // Set auto_fix from command-line arguments
        let mut default_tool_config = default_tool_config;
        default_tool_config.auto_fix = Some(args.auto_fix);

        // Create a tool runner
        let tool_runner = ToolRunner::new();

        // Create our neon status display
        let mut status_display = terminal::NeonDisplay::new();

        // Store captured outputs for display at the end - only if in verbose mode
        let mut captured_outputs = Vec::new();

        // Run each linter and collect results
        let mut all_results = Vec::new();
        let mut tool_statuses = Vec::new();
        let mut total_issues = 0;

        // Process the scan and collect results
        for linter in &tools {
            // Get tool-specific config or use default
            let mut config_tool_config = config
                .tools
                .get(linter.name())
                .cloned()
                .unwrap_or_else(|| default_tool_config.clone());

            // Set auto_fix from command-line arguments
            config_tool_config.auto_fix = Some(args.auto_fix);

            // Convert to the correct ToolConfig type for the runner
            let config_for_runner = convert_tool_config(&config_tool_config);

            // Create a status for this tool
            let language = format!("{:?}", linter.language());
            let tool_type = format!("{:?}", linter.tool_type());
            let spinner_index =
                status_display.add_tool_status(linter.name(), &language, &tool_type);

            // Execute the tool - without logging the execution details unless in verbose mode
            if self.verbosity >= Verbosity::Verbose {
                debug!("Running linter: {} on {} files", linter.name(), files.len());
            }

            // Execute the tool and capture results
            let tool_results = tool_runner
                .run_tools(vec![linter.clone()], &files, &config_for_runner)
                .await;

            // Process results and update the status
            for result in tool_results {
                match result {
                    Ok(result) => {
                        let issues_count = result.issues.len();
                        total_issues += issues_count;

                        // Only save stdout/stderr in verbose mode
                        if self.verbosity >= Verbosity::Verbose
                            && (result.stdout.is_some() || result.stderr.is_some())
                        {
                            captured_outputs.push((
                                linter.name().to_string(),
                                result.stdout.clone().unwrap_or_default(),
                                result.stderr.clone().unwrap_or_default(),
                            ));
                        }

                        if issues_count > 0 {
                            status_display.finish_spinner(
                                spinner_index,
                                format!(
                                    "{} 「{}」",
                                    linter.name(),
                                    format!("{} issues detected", issues_count).red()
                                ),
                            );
                        } else {
                            status_display.finish_spinner(
                                spinner_index,
                                format!("{} 「{}」", linter.name(), "system clean".green()),
                            );
                        }

                        all_results.push(result);
                    }
                    Err(err) => {
                        status_display.finish_spinner(
                            spinner_index,
                            format!("{} 「{}」", linter.name(), "execution failed".red()),
                        );

                        // Save error for later display only if verbose
                        if self.verbosity >= Verbosity::Verbose {
                            captured_outputs.push((
                                linter.name().to_string(),
                                String::new(),
                                format!("ERROR: {}", err),
                            ));
                        }

                        if self.verbosity >= Verbosity::Normal {
                            debug!("Error running {}: {}", linter.name(), err);
                            tool_statuses.push(format!("❌ {} failed: {}", linter.name(), err));
                        }
                    }
                }
            }
        }

        // Finish the status display
        status_display.finish(total_issues);

        // A moment to appreciate the UI
        std::thread::sleep(std::time::Duration::from_millis(300));

        // Now display the captured outputs if in verbose mode
        if self.verbosity >= Verbosity::Verbose && !captured_outputs.is_empty() {
            println!("\nraw tool output:");

            for (tool_name, stdout, stderr) in captured_outputs {
                // Only show non-empty output
                if !stdout.trim().is_empty() || !stderr.trim().is_empty() {
                    println!("\n{}", tool_name.bright_magenta());
                    println!("{}", "─".repeat(tool_name.len()).bright_blue());

                    if !stdout.trim().is_empty() {
                        println!("{}", stdout);
                    }

                    if !stderr.trim().is_empty() {
                        println!("{}", stderr);
                    }
                }
            }
        }

        // Format and display results
        if !all_results.is_empty() {
            // Print any error messages we collected during tool execution
            for status in tool_statuses {
                eprintln!("{}", status);
            }

            println!("\ndetailed analysis:");

            // Format and print the actual results
            let results_output = self
                .output_formatter
                .format_results(&all_results, &config.output);
            println!("{}", results_output);

            // Display summary
            let summary = self.output_formatter.format_summary(&all_results);
            println!("{}", summary);
        } else if self.verbosity >= Verbosity::Normal {
            println!("\nall systems operational - no issues detected");
        }

        Ok(())
    }

    // Helper methods moved from app.rs

    /// Detect project information from the provided paths
    fn detect_project(&self, paths: &[PathBuf]) -> Result<ProjectInfo, SirenError> {
        // Use the first path or current dir if empty
        let dir = paths
            .first()
            .map(|p| p.as_path())
            .unwrap_or_else(|| Path::new("."));

        self.detector.detect(dir)
    }

    /// Detect project information with specific path patterns
    fn detect_project_with_patterns(
        &self,
        dir: &Path,
        patterns: &[String],
    ) -> Result<ProjectInfo, SirenError> {
        self.detector.detect_with_patterns(dir, patterns)
    }

    /// Select appropriate tools for checking based on project info and arguments
    fn select_tools_for_check(
        &self,
        project_info: &ProjectInfo,
        args: &CheckArgs,
        _config: &SirenConfig,
    ) -> Result<Vec<Arc<dyn LintTool>>, SirenError> {
        let mut selected_tools = Vec::new();

        // Debug - Print all languages detected only in verbose mode
        if self.verbosity >= Verbosity::Verbose {
            debug!("Detected languages: {:?}", project_info.languages);
        }

        // If specific tools are requested, use those
        if let Some(tool_names) = &args.tools {
            if self.verbosity >= Verbosity::Verbose {
                debug!("Specific tools requested: {:?}", tool_names);
            }
            for name in tool_names {
                if let Some(tool) = self.tool_registry.get_tool_by_name(name) {
                    if self.verbosity >= Verbosity::Verbose {
                        debug!("Found tool '{}', available: {}", name, tool.is_available());
                    }
                    selected_tools.push(tool);
                } else if self.verbosity >= Verbosity::Normal {
                    warn!("⚠️ Tool '{}' not found", name);
                }
            }
            return Ok(selected_tools);
        }

        // If specific tool types are requested, use those
        if let Some(type_names) = &args.tool_types {
            for type_name in type_names {
                let tool_type = match type_name.to_lowercase().as_str() {
                    "formatter" => ToolType::Formatter,
                    "linter" => ToolType::Linter,
                    "typechecker" => ToolType::TypeChecker,
                    "fixer" => ToolType::Fixer,
                    _ => {
                        if self.verbosity >= Verbosity::Normal {
                            eprintln!("⚠️ Unknown tool type: {}", type_name);
                        }
                        continue;
                    }
                };

                let tools = self.tool_registry.get_tools_by_type(tool_type);
                selected_tools.extend(tools);
            }
            return Ok(selected_tools);
        }

        // Otherwise, select tools based on detected languages
        if self.verbosity >= Verbosity::Verbose {
            eprintln!("Selecting tools based on detected languages");
        }
        for language in &project_info.languages {
            if self.verbosity >= Verbosity::Verbose {
                debug!("Getting tools for language: {:?}", language);
            }
            let language_tools = self.tool_registry.get_tools_for_language(*language);
            if self.verbosity >= Verbosity::Verbose {
                debug!("Found {} tools for {:?}", language_tools.len(), language);
            }

            // For general check, prefer linters and type checkers
            let filtered_tools: Vec<_> = language_tools
                .into_iter()
                .filter(|tool| {
                    let tool_type = tool.tool_type();
                    let available = tool.is_available();
                    if self.verbosity >= Verbosity::Verbose {
                        debug!(
                            "  - Tool: {}, Type: {:?}, Available: {}",
                            tool.name(),
                            tool_type,
                            available
                        );
                    }
                    (tool_type == ToolType::Linter || tool_type == ToolType::TypeChecker)
                        && available
                })
                .collect();

            if self.verbosity >= Verbosity::Verbose {
                debug!("Selected {} tools after filtering", filtered_tools.len());
            }
            selected_tools.extend(filtered_tools);
        }

        // If strict mode, add additional strict tools
        if args.strict {
            // TODO: Add strictness logic here
        }

        // Debug - Print selected tools only in verbose mode
        if self.verbosity >= Verbosity::Verbose {
            debug!("Final selected tools: {}", selected_tools.len());
            for tool in &selected_tools {
                debug!("  - {}, Available: {}", tool.name(), tool.is_available());
            }
        }

        Ok(selected_tools)
    }
}

// Helper function to convert from config::ToolConfig to models::tools::ToolConfig
fn convert_tool_config(config: &crate::config::ToolConfig) -> ToolConfig {
    ToolConfig {
        enabled: config.enabled,
        extra_args: config.extra_args.clone().unwrap_or_default(),
        env_vars: HashMap::new(),
        executable_path: config
            .config_file
            .clone()
            .map(|p| p.to_string_lossy().to_string()),
        report_level: None,
        auto_fix: config.auto_fix.unwrap_or(false),
        check: config.check.unwrap_or(false),
    }
}
