use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::cli::{FixArgs, FormatArgs};
use crate::config::{SirenConfig, ToolConfig as ConfigToolConfig};
use crate::detection::ProjectDetector;
use crate::errors::SirenError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::ToolType;
use crate::output::OutputFormatter;
use crate::runner::ToolRunner;
use crate::tools::ToolRegistry;

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
}

impl<D, R, O> FixCommand<D, R, O>
where
    D: ProjectDetector + Clone,
    R: ToolRegistry + Clone,
    O: OutputFormatter + Clone,
{
    /// Create a new fix command handler
    pub fn new(detector: D, tool_registry: R, output_formatter: O) -> Self {
        Self {
            detector,
            tool_registry,
            output_formatter,
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
            println!("💅 Running format before fix...");

            // Create and run a format command first
            let format_command = crate::commands::FormatCommand::new(
                self.detector.clone(),
                self.tool_registry.clone(),
                self.output_formatter.clone(),
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

        // Clone paths from args to avoid ownership issues
        let args_paths = args.paths.clone();

        // Combine paths from the Cli struct and FixArgs
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

        // Prepare paths for detection
        let paths_to_detect = if all_paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            all_paths.clone()
        };

        // Detect project information
        let project_info = if !patterns.is_empty() {
            self.detector.detect_with_patterns(dir, &patterns)?
        } else {
            self.detector.detect(&paths_to_detect)?
        };

        // Always display detected project info if we didn't run format first
        if !args.format {
            let info_output = self.output_formatter.format_detection(&project_info);
            println!("{}", info_output);
        }

        // Print detected languages
        println!(
            "🔍 Looking for fixers for languages: {:?}",
            project_info.languages
        );

        // Select appropriate fixing tools
        let mut fixers = Vec::new();
        for language in &project_info.languages {
            println!("  Looking for fixers for {:?}...", language);
            let language_fixers = self
                .tool_registry
                .get_tools_for_language_and_type(*language, ToolType::Fixer);

            println!(
                "  Found {} fixers for {:?}",
                language_fixers.len(),
                language
            );
            for fixer in language_fixers {
                println!(
                    "    - {} (available: {})",
                    fixer.name(),
                    fixer.is_available()
                );
                fixers.push(fixer);
            }
        }

        if fixers.is_empty() {
            println!("⚠️ No fixers found for the detected languages. Available tools:");

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

            return Ok(());
        }

        // Get files to fix
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

        println!("📂 Found {} files to fix:", files.len());
        for file in &files {
            println!("  - {}", file.display());
        }

        if files.is_empty() {
            println!("⚠️ No files found to fix!");
            return Ok(());
        }

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

        println!("🔧 Running {} fixers...", available_fixers.len());
        let results = tool_runner
            .run_tools(available_fixers, &files, &default_config)
            .await;

        // Process results
        let mut all_results = Vec::new();
        for result in results {
            match result {
                Ok(result) => {
                    let issue_count = result.issues.len();
                    println!(
                        "  ✅ {} completed with {} issues fixed",
                        result.tool_name, issue_count
                    );

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
}
