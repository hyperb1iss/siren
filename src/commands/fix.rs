use std::collections::HashMap;
use std::path::PathBuf;

use crate::cli::{FixArgs, FormatArgs, Verbosity};
use crate::config::{SirenConfig, ToolConfig as ConfigToolConfig};
use crate::detection::ProjectDetector;
use crate::errors::SirenError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::ToolType;
use crate::output::OutputFormatter;
use crate::runner::ToolRunner;
use crate::tools::ToolRegistry;
use crate::utils::path_manager::PathManager;

/// Command handler for the fix command
pub struct FixCommand<D, R, O>
where
    D: ProjectDetector + Clone,
    R: ToolRegistry + Clone,
    O: OutputFormatter + Clone,
{
    detector: D,
    tool_registry: R,
    output_formatter: O,
    verbosity: Verbosity,
}

impl<D, R, O> FixCommand<D, R, O>
where
    D: ProjectDetector + Clone,
    R: ToolRegistry + Clone,
    O: OutputFormatter + Clone,
{
    /// Create a new fix command handler
    pub fn new(detector: D, tool_registry: R, output_formatter: O, verbosity: Verbosity) -> Self {
        Self {
            detector,
            tool_registry,
            output_formatter,
            verbosity,
        }
    }

    /// Execute the fix command
    pub async fn execute(
        &self,
        args: FixArgs,
        paths: Vec<PathBuf>,
        git_modified_only: bool,
        config: &SirenConfig,
    ) -> Result<(), SirenError> {
        // First run the format command if requested
        if args.format {
            if self.verbosity >= Verbosity::Normal {
                println!("💅 Running format before fix...");
            }

            // Create and run a format command first
            let format_command = crate::commands::FormatCommand::new(
                self.detector.clone(),
                self.tool_registry.clone(),
                self.output_formatter.clone(),
                self.verbosity,
            );

            // Create FormatArgs without the check option
            let format_args = FormatArgs {
                check: false,
                tools: args.tools.clone(),
                paths: args.paths.clone(),
            };

            // Run the format command
            format_command
                .execute(format_args, paths.clone(), git_modified_only, config)
                .await?;
        }

        // Combine paths from the Cli struct and FixArgs
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
        if self.verbosity >= Verbosity::Normal && !args.format {
            let info_output = self.output_formatter.format_detection(&project_info);
            println!("{}", info_output);
        }

        // Print detected languages based on verbosity
        if self.verbosity >= Verbosity::Normal {
            println!("🔍 Detected languages: {:?}", project_info.languages);
        }

        // Select appropriate fixing tools
        let mut fixers = Vec::new();
        for language in &project_info.languages {
            if self.verbosity >= Verbosity::Normal {
                println!("  Looking for fixers for {:?}...", language);
            }

            let language_fixers = self
                .tool_registry
                .get_tools_for_language_and_type(*language, ToolType::Fixer);

            if self.verbosity >= Verbosity::Normal {
                println!(
                    "  Found {} fixers for {:?}",
                    language_fixers.len(),
                    language
                );

                for fixer in &language_fixers {
                    println!(
                        "    - {} (available: {})",
                        fixer.name(),
                        fixer.is_available()
                    );
                }
            }

            for fixer in language_fixers {
                if fixer.is_available() {
                    fixers.push(fixer);
                } else if self.verbosity >= Verbosity::Normal {
                    println!("⚠️ Skipping unavailable fixer: {}", fixer.name());
                }
            }
        }

        if fixers.is_empty() {
            println!("⚠️ No fixers found for the detected languages.");

            if self.verbosity >= Verbosity::Verbose {
                println!("Available tools:");
                // Print all available tools to help debug
                for language in &project_info.languages {
                    let all_tools = self.tool_registry.get_tools_for_language(*language);
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
        let files_to_fix = path_manager.get_all_files().to_vec();

        // Debug output for files to fix
        if self.verbosity >= Verbosity::Normal {
            println!("📂 Found {} files/directories to fix:", files_to_fix.len());

            if self.verbosity >= Verbosity::Verbose {
                for file in &files_to_fix {
                    println!("  - {}", file.display());
                }
            }
        }

        if files_to_fix.is_empty() {
            println!("⚠️ No files found to fix!");
            return Ok(());
        }

        // Create a map to store paths for each fixer
        let mut fixer_paths_map: HashMap<String, Vec<PathBuf>> = HashMap::new();

        // Get default tool config
        let default_tool_config = config.tools.get("default").cloned().unwrap_or_default();

        // Create a tool runner
        let tool_runner = ToolRunner::new();

        // Prepare all available fixers
        let available_fixers: Vec<_> = fixers.into_iter().filter(|f| f.is_available()).collect();

        if available_fixers.is_empty() {
            println!("⚠️ No available fixers found for the detected languages.");
            return Ok(());
        }

        // Setup default tool configuration
        let mut default_config = self.convert_tool_config(&default_tool_config);
        default_config.auto_fix = true; // Ensure auto_fix is enabled for fixers

        if self.verbosity >= Verbosity::Normal {
            println!("🔧 Running {} fixers...", available_fixers.len());
        }

        // Get optimized paths for each fixer
        let mut fixer_specific_paths = Vec::new();
        for fixer in &available_fixers {
            let paths = path_manager.get_optimized_paths_for_tool(fixer.as_ref());
            fixer_paths_map.insert(fixer.name().to_string(), paths.clone());
            fixer_specific_paths.push(paths);
        }

        // Run all fixers with their specific paths
        let results = tool_runner
            .run_tools_with_specific_paths(
                available_fixers.clone(),
                fixer_specific_paths,
                &default_config,
            )
            .await;

        // Process results
        let mut all_results = Vec::new();
        for result in results {
            match result {
                Ok(result) => {
                    let issue_count = result.issues.len();

                    if self.verbosity >= Verbosity::Normal {
                        println!(
                            "  ✅ {} completed with {} issues fixed",
                            result.tool_name, issue_count
                        );
                    }

                    // Just add the result to all_results for formatting
                    all_results.push(result);
                }
                Err(err) => {
                    println!("  ❌ Error running fixer: {}", err);
                }
            }
        }

        // Format and display results
        if !all_results.is_empty() {
            let results_output = self
                .output_formatter
                .format_results(&all_results, &config.output);
            println!("{}", results_output);

            // Display summary
            let summary = self.output_formatter.format_summary(&all_results);
            println!("\n{}", summary);
        } else {
            println!("✨ No issues to fix!");
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
