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
            if paths.is_empty() {
                // If no paths provided at all, use current directory
                vec![PathBuf::from(".")]
            } else {
                paths.clone()
            }
        } else {
            args_paths
        };

        // Get project root directory
        let dir = all_paths
            .first()
            .map(|p| p.as_path())
            .unwrap_or_else(|| Path::new("."));

        // Detect project information
        let project_info = self.detect_project(&all_paths)?;

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

        // Collect files to check
        let files_to_check = if git_modified_only {
            // Get files modified in git
            let git_files = crate::utils::get_git_modified_files(dir)?;

            // Filter git files to only include those that match our paths
            if all_paths.len() == 1 && all_paths[0] == PathBuf::from(".") {
                // If only the current directory is specified, use all git files
                git_files
            } else {
                git_files
                    .into_iter()
                    .filter(|file| {
                        all_paths.iter().any(|path| {
                            if path.is_dir() {
                                file.starts_with(path)
                            } else {
                                file == path
                            }
                        })
                    })
                    .collect()
            }
        } else {
            let mut all_files = Vec::new();

            // If only the current directory is specified, scan it
            if all_paths.len() == 1 && all_paths[0] == PathBuf::from(".") {
                let dir_files = crate::utils::collect_files_with_gitignore(Path::new("."))?;
                all_files.extend(dir_files);
            } else {
                for path in &all_paths {
                    if path.is_file() {
                        // If it's a specific file, just add it directly
                        all_files.push(path.clone());
                    } else if path.is_dir() {
                        // If it's a directory, collect files from it
                        let dir_files = crate::utils::collect_files_with_gitignore(path)?;
                        all_files.extend(dir_files);
                    }
                }
            }

            all_files
        };

        // Debug output for files to check
        if self.verbosity >= Verbosity::Verbose {
            println!("\nFiles to check: {}", files_to_check.len());
            for file in &files_to_check {
                println!("  - {}", file.display());
            }
            println!();
        }

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

        // Group tools by their config hash to run tools with the same config together
        let mut tool_groups: HashMap<String, Vec<(Arc<dyn LintTool>, usize)>> = HashMap::new();

        // Set up all tools first and group them by config
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

            // Create a simple hash of the config to use as a key for grouping
            // This is a simplification - in a real implementation we might want a better way to compare configs
            let config_key = format!("{:?}", config_for_runner);

            // Create a status for this tool
            let language = format!("{:?}", linter.language());
            let tool_type = format!("{:?}", linter.tool_type());
            let spinner_index =
                status_display.add_tool_status(linter.name(), &language, &tool_type);

            // Execute the tool - without logging the execution details unless in verbose mode
            if self.verbosity >= Verbosity::Verbose {
                debug!(
                    "Running linter: {} on {} files",
                    linter.name(),
                    files_to_check.len()
                );
            }

            // Group tools by their config
            tool_groups
                .entry(config_key)
                .or_default()
                .push((linter.clone(), spinner_index));
        }

        // Process results and update the status
        let mut all_results = Vec::new();
        let mut tool_statuses = Vec::new();
        let mut total_issues = 0;

        // Run each group of tools with the same config in parallel
        for (_, group) in tool_groups {
            // Extract tools and spinner indices
            let group_tools: Vec<_> = group.iter().map(|(tool, _)| tool.clone()).collect();
            let group_spinner_indices: Vec<_> = group.iter().map(|(_, idx)| *idx).collect();

            // Skip empty groups
            if group_tools.is_empty() {
                continue;
            }

            // Get the config for this group (they all have the same config)
            let linter = &group_tools[0];
            let mut config_tool_config = config
                .tools
                .get(linter.name())
                .cloned()
                .unwrap_or_else(|| default_tool_config.clone());
            config_tool_config.auto_fix = Some(args.auto_fix);
            let config_for_runner = convert_tool_config(&config_tool_config);

            // Run all tools in this group in parallel
            let group_results = tool_runner
                .run_tools(group_tools.clone(), &files_to_check, &config_for_runner)
                .await;

            // Process results for this group
            for (i, result) in group_results.into_iter().enumerate() {
                let linter = &group_tools[i];
                let spinner_index = group_spinner_indices[i];

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

        // Print the results
        if !all_results.is_empty() {
            // Print the results
            println!(
                "{}",
                self.output_formatter
                    .format_results(&all_results, &config.output)
            );

            // Print the summary
            println!("{}", self.output_formatter.format_summary(&all_results));
        } else {
            println!("\nNo issues found!");
        }

        Ok(())
    }

    // Helper methods moved from app.rs

    /// Detect project information from the provided paths
    fn detect_project(&self, paths: &[PathBuf]) -> Result<ProjectInfo, SirenError> {
        self.detector.detect(paths)
    }

    /// Detect project information with specific path patterns
    fn detect_project_with_patterns(
        &self,
        dir: &Path,
        patterns: &[String],
    ) -> Result<ProjectInfo, SirenError> {
        // This method is kept for backward compatibility but is no longer used
        // in the execute method. We now handle paths directly in the detect method.
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
