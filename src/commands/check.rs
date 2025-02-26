use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::cli::{CheckArgs, Verbosity};
use crate::config::{SirenConfig, ToolConfig as ConfigToolConfig};
use crate::detection::ProjectDetector;
use crate::errors::SirenError;
use crate::models::tools::ToolConfig;
use crate::models::ToolType;
use crate::output::{terminal, OutputFormatter};
use crate::runner::ToolRunner;
use crate::tools::{LintTool, ToolRegistry};
use crate::utils::file_selection;
use colored::*;
use log::debug;

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

        // Detect project information
        let (project_info, detected_files) = self.detector.detect(&all_paths)?;

        // Display detected project info based on verbosity
        if self.verbosity >= Verbosity::Normal {
            let info_output = self.output_formatter.format_detection(&project_info);
            println!("{}", info_output);
        }

        // Print detected languages based on verbosity
        if self.verbosity >= Verbosity::Normal {
            println!("🔍 Detected languages: {:?}", project_info.languages);
        }

        // Select appropriate linting tools
        let mut linters = Vec::new();
        for language in &project_info.languages {
            if self.verbosity >= Verbosity::Normal {
                println!("  Looking for linters for {:?}...", language);
            }

            let language_linters = self
                .tool_registry
                .get_tools_for_language_and_type(*language, ToolType::Linter);

            if self.verbosity >= Verbosity::Normal {
                println!(
                    "  Found {} linters for {:?}",
                    language_linters.len(),
                    language
                );

                for linter in &language_linters {
                    println!(
                        "    - {} (available: {})",
                        linter.name(),
                        linter.is_available()
                    );
                }
            }

            for linter in language_linters {
                if linter.is_available() {
                    linters.push(linter);
                } else if self.verbosity >= Verbosity::Normal {
                    println!("⚠️ Skipping unavailable linter: {}", linter.name());
                }
            }
        }

        if linters.is_empty() {
            println!("⚠️ No linters found for the detected languages.");
            return Ok(());
        }

        // Use the files collected during detection if not using git-modified-only
        let files_to_check = if git_modified_only {
            // For git-modified-only, we still need to use the file_selection utility
            file_selection::collect_files_to_process(&all_paths, git_modified_only)?
        } else {
            // Reuse the files collected during detection
            detected_files
        };

        // Debug output for files to check
        if self.verbosity >= Verbosity::Normal {
            println!("📂 Found {} files to check:", files_to_check.len());

            if self.verbosity >= Verbosity::Verbose {
                for file in &files_to_check {
                    println!("  - {}", file.display());
                }
            }
        }

        // Count files by language for information purposes
        let _total_files: usize = project_info.file_counts.values().sum();
        let _files_by_language = project_info.file_counts.clone();

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

        // Define a type for tool groups to simplify the complex type
        type ToolGroup = Vec<(Arc<dyn LintTool>, usize)>;

        // Group tools by their config hash to run tools with the same config together
        let mut tool_groups: HashMap<String, ToolGroup> = HashMap::new();

        // Set up all tools first and group them by config
        for linter in &linters {
            // Get tool-specific config or use default
            let mut config_tool_config = config
                .tools
                .get(linter.name())
                .cloned()
                .unwrap_or_else(|| default_tool_config.clone());

            // Set auto_fix from command-line arguments
            config_tool_config.auto_fix = Some(args.auto_fix);

            // Convert to the correct ToolConfig type for the runner
            let config_for_runner = self.convert_tool_config(&config_tool_config);

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
            let config_for_runner = self.convert_tool_config(&config_tool_config);

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

    /// Convert from ConfigToolConfig to ToolConfig
    fn convert_tool_config(&self, config: &ConfigToolConfig) -> ToolConfig {
        ToolConfig {
            enabled: config.enabled,
            extra_args: config.extra_args.clone().unwrap_or_default(),
            env_vars: std::collections::HashMap::new(),
            executable_path: None,
            report_level: None,
            auto_fix: config.auto_fix.unwrap_or(false),
            check: config.check.unwrap_or(false),
        }
    }
}
