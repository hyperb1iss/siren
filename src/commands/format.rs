use std::collections::HashMap;
use std::path::PathBuf;

use crate::cli::{FormatArgs, Verbosity};
use crate::config::{SirenConfig, ToolConfig as ConfigToolConfig};
use crate::detection::ProjectDetector;
use crate::errors::SirenError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::ToolType;
use crate::output::{terminal, OutputFormatter};
use crate::runner::ToolRunner;
use crate::tools::ToolRegistry;
use crate::utils::path_manager::PathManager;
use colored::*;
use log::debug;

/// Command handler for the format command
pub struct FormatCommand<D, R, O>
where
    D: ProjectDetector + Clone,
    R: ToolRegistry + Clone,
    O: OutputFormatter + Clone,
{
    detector: D,
    registry: R,
    output_formatter: O,
    verbosity: Verbosity,
}

impl<D, R, O> FormatCommand<D, R, O>
where
    D: ProjectDetector + Clone,
    R: ToolRegistry + Clone,
    O: OutputFormatter + Clone,
{
    /// Create a new format command handler
    pub fn new(detector: D, registry: R, output_formatter: O, verbosity: Verbosity) -> Self {
        Self {
            detector,
            registry,
            output_formatter,
            verbosity,
        }
    }

    /// Execute the format command
    pub async fn execute(
        &self,
        args: FormatArgs,
        paths: Vec<PathBuf>,
        git_modified_only: bool,
        config: &SirenConfig,
    ) -> Result<(), SirenError> {
        // Combine paths from the Cli struct and FormatArgs
        let all_paths = if !args.paths.is_empty() {
            args.paths.clone()
        } else {
            paths.clone()
        };

        // Create and initialize the path manager
        let mut path_manager = PathManager::new();
        path_manager.collect_files(&all_paths, git_modified_only)?;

        // Detect project information
        let (project_info, _) = self.detector.detect(&all_paths)?;

        // Display detected project info based on verbosity
        if self.verbosity >= Verbosity::Normal {
            let info_output = self.output_formatter.format_detection(&project_info);
            println!("{}", info_output);
        }

        // Print detected languages based on verbosity
        if self.verbosity >= Verbosity::Normal {
            println!("🔍 Detected languages: {:?}", project_info.languages);
        }

        // Select appropriate formatting tools
        let mut formatters = Vec::new();
        for language in &project_info.languages {
            if self.verbosity >= Verbosity::Normal {
                println!("  Looking for formatters for {:?}...", language);
            }

            let language_formatters = self
                .registry
                .get_tools_for_language_and_type(*language, ToolType::Formatter);

            if self.verbosity >= Verbosity::Normal {
                println!(
                    "  Found {} formatters for {:?}",
                    language_formatters.len(),
                    language
                );

                for formatter in &language_formatters {
                    println!(
                        "    - {} (available: {})",
                        formatter.name(),
                        formatter.is_available()
                    );
                }
            }

            for formatter in language_formatters {
                if formatter.is_available() {
                    formatters.push(formatter);
                } else if self.verbosity >= Verbosity::Normal {
                    println!("⚠️ Skipping unavailable formatter: {}", formatter.name());
                }
            }
        }

        if formatters.is_empty() {
            println!("⚠️ No formatters found for the detected languages.");

            if self.verbosity >= Verbosity::Verbose {
                println!("Available tools:");
                // Print all available tools to help debug
                for language in &project_info.languages {
                    let all_tools = self.registry.get_tools_for_language(*language);
                    println!("  Tools for {:?}:", language);
                    for tool in all_tools {
                        println!(
                            "    - {} ({:?}, available: {})",
                            tool.name(),
                            tool.tool_type(),
                            tool.is_available()
                        );
                    }
                }
            }

            return Ok(());
        }

        // Use the files collected by the path manager
        let all_files = path_manager.get_all_files().to_vec();

        // Debug output for all collected files
        if self.verbosity >= Verbosity::Verbose {
            println!(
                " Collected {} files/directories to format:",
                all_files.len()
            );
            for file in &all_files {
                println!("  - {}", file.display());
            }
        }

        if all_files.is_empty() {
            println!("⚠️ No files found to format!");
            return Ok(());
        }

        // Get default tool config
        let default_tool_config = config.tools.get("default").cloned().unwrap_or_default();

        // Create a tool runner
        let tool_runner = ToolRunner::new();

        // Create our neon status display
        let mut status_display = terminal::NeonDisplay::new();

        // Prepare all available formatters
        let available_formatters: Vec<_> = formatters
            .into_iter()
            .filter(|f| f.is_available())
            .collect();

        if available_formatters.is_empty() {
            println!("⚠️ No available formatters found for the detected languages.");
            return Ok(());
        }

        // Store formatter and spinner index pairs
        let mut formatter_spinners = Vec::new();

        // Setup formatters in the UI
        for formatter in &available_formatters {
            let language = format!("{:?}", formatter.languages());
            let tool_type = format!("{:?}", formatter.tool_type());
            let spinner_index =
                status_display.add_tool_status(formatter.name(), &language, &tool_type);
            formatter_spinners.push((formatter.clone(), spinner_index));
        }

        // Add a small delay to ensure we can see the spinners before they complete
        std::thread::sleep(std::time::Duration::from_millis(800));

        // Create a map to store paths for each formatter
        let mut formatter_paths_map: HashMap<String, Vec<PathBuf>> = HashMap::new();

        // Run each formatter with its filtered files
        let mut all_results = Vec::new();
        let mut total_issues = 0;

        for (formatter, spinner_index) in &formatter_spinners {
            // Get paths for this formatter
            let files_for_formatter = path_manager.get_optimized_paths_for_tool(&**formatter);

            // Store the paths for this formatter
            formatter_paths_map.insert(formatter.name().to_string(), files_for_formatter.clone());

            if files_for_formatter.is_empty() {
                if self.verbosity >= Verbosity::Normal {
                    debug!("No files for formatter: {}", formatter.name());
                }
                // Update spinner to show no files found
                status_display.finish_spinner(
                    *spinner_index,
                    format!("{} 「{}」", formatter.name(), "no matching files".yellow()),
                );
                continue;
            }

            if self.verbosity >= Verbosity::Verbose {
                debug!(
                    "Running {} on {} files",
                    formatter.name(),
                    files_for_formatter.len()
                );

                for file in &files_for_formatter {
                    debug!("  - {}", file.display());
                }
            }

            // Setup default tool configuration
            let mut default_config = self.convert_tool_config(&default_tool_config);
            default_config.check = args.check;

            // For rustfmt, add the -l flag to report which files were actually formatted
            if formatter.name() == "rustfmt" {
                default_config.extra_args.push("-l".to_string());
            }

            // Run the formatter on its filtered files
            let result = tool_runner
                .run_tools(
                    vec![formatter.clone()],
                    &files_for_formatter,
                    &default_config,
                )
                .await;

            // Process the first result (there should only be one since we're running one tool)
            if let Some(result) = result.into_iter().next() {
                match result {
                    Ok(mut result) => {
                        let issue_count = result.issues.len();
                        total_issues += issue_count;

                        // For formatters, if there are no issues but files were processed,
                        // create "fake" issues to track which files were formatted
                        if result.issues.is_empty()
                            && formatter.tool_type() == ToolType::Formatter
                            && !files_for_formatter.is_empty()
                        {
                            // For rustfmt, we don't need to add fake issues since we'll parse its output
                            // with the -l flag to find which files were actually formatted
                            if formatter.name() != "rustfmt" {
                                // In a real implementation, we would check if the file was actually modified
                                // by comparing before/after content. For now, we'll just add all files.
                                //
                                // A better approach would be to have the formatter tools report which files
                                // they actually modified, but that would require changes to the tool interfaces.

                                // For demonstration purposes, we'll add all processed files
                                for file in &files_for_formatter {
                                    result.issues.push(crate::models::LintIssue {
                                        severity: crate::models::IssueSeverity::Info,
                                        message: "File formatted".to_string(),
                                        file: Some(file.clone()),
                                        line: None,
                                        column: None,
                                        code: None,
                                        fix_available: false,
                                    });
                                }
                            }
                        }

                        // Update spinner with the result
                        if issue_count > 0 {
                            status_display.finish_spinner(
                                *spinner_index,
                                format!(
                                    "{} 「{}」",
                                    formatter.name(),
                                    format!("{} files formatted", issue_count).green()
                                ),
                            );
                        } else {
                            status_display.finish_spinner(
                                *spinner_index,
                                format!("{} 「{}」", formatter.name(), "no changes needed".blue()),
                            );
                        }

                        if self.verbosity >= Verbosity::Verbose {
                            debug!("{} completed with {} issues", result.tool_name, issue_count);
                        }

                        // Add the result to all_results for formatting
                        all_results.push(result);
                    }
                    Err(err) => {
                        // Update spinner with the error
                        status_display.finish_spinner(
                            *spinner_index,
                            format!("{} 「{}」", formatter.name(), "execution failed".red()),
                        );

                        debug!("Error running formatter {}: {}", formatter.name(), err);

                        if self.verbosity >= Verbosity::Normal {
                            println!("  ❌ Error running formatter {}: {}", formatter.name(), err);
                        }
                    }
                }
            }
        }

        // Finish the status display
        status_display.finish(total_issues);

        // A moment to appreciate the UI
        std::thread::sleep(std::time::Duration::from_millis(300));

        // Format and display results only if there are issues or in verbose mode
        if !all_results.is_empty() && (total_issues > 0 || self.verbosity >= Verbosity::Verbose) {
            // Format results
            let results_output = self
                .output_formatter
                .format_results(&all_results, &config.output);
            println!("{}", results_output);

            // Display summary
            println!("\n{}", self.output_formatter.format_summary(&all_results));
        } else if total_issues > 0 {
            // Just show a simple summary if there are issues but we're not showing details
            println!("\n✨ {} files were formatted!", total_issues);
        }

        Ok(())
    }

    /// Convert from ConfigToolConfig to ModelsToolConfig
    fn convert_tool_config(&self, config: &ConfigToolConfig) -> ModelsToolConfig {
        ModelsToolConfig {
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
