use std::path::{Path, PathBuf};

use crate::cli::FormatArgs;
use crate::config::{SirenConfig, ToolConfig as ConfigToolConfig};
use crate::detection::ProjectDetector;
use crate::errors::SirenError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::ToolType;
use crate::output::OutputFormatter;
use crate::runner::ToolRunner;
use crate::tools::ToolRegistry;

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
}

impl<D, R, O> FormatCommand<D, R, O>
where
    D: ProjectDetector + Clone,
    R: ToolRegistry + Clone,
    O: OutputFormatter + Clone,
{
    /// Create a new format command handler
    pub fn new(detector: D, registry: R, output_formatter: O) -> Self {
        Self {
            detector,
            registry,
            output_formatter,
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
        // Clone paths from args to avoid ownership issues
        let args_paths = args.paths.clone();

        // Combine paths from the Cli struct and FormatArgs
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
            self.detector.detect_with_patterns(dir, &patterns)?
        } else {
            let paths_to_detect = if all_paths.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                all_paths.clone()
            };
            self.detector
                .detect(paths_to_detect.first().unwrap().as_path())?
        };

        // Always display detected project info
        let info_output = self.output_formatter.format_detection(&project_info);
        println!("{}", info_output);

        // Print detected languages
        println!("🔍 Detected languages: {:?}", project_info.languages);

        // Select appropriate formatting tools
        let mut formatters = Vec::new();
        for language in &project_info.languages {
            println!("  Looking for formatters for {:?}...", language);
            let language_formatters = self
                .registry
                .get_tools_for_language_and_type(*language, ToolType::Formatter);

            println!(
                "  Found {} formatters for {:?}",
                language_formatters.len(),
                language
            );
            for formatter in language_formatters {
                println!(
                    "    - {} (available: {})",
                    formatter.name(),
                    formatter.is_available()
                );
                if formatter.is_available() {
                    formatters.push(formatter);
                } else {
                    println!("⚠️ Skipping unavailable formatter: {}", formatter.name());
                }
            }
        }

        if formatters.is_empty() {
            println!("⚠️ No formatters found for the detected languages. Available tools:");

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

            return Ok(());
        }

        // Get files to format
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

        println!("📂 Found {} files to format:", files.len());
        for file in &files {
            println!("  - {}", file.display());
        }

        if files.is_empty() {
            println!("⚠️ No files found to format!");
            return Ok(());
        }

        // Default tool config
        let default_tool_config = ConfigToolConfig::default();

        // Create a tool runner
        let tool_runner = ToolRunner::new();

        // Prepare all available formatters
        let available_formatters: Vec<_> = formatters
            .into_iter()
            .filter(|f| f.is_available())
            .collect();

        if available_formatters.is_empty() {
            println!("⚠️ No available formatters found for the detected languages.");
            return Ok(());
        }

        // Setup default tool configuration
        let mut default_config = self.convert_tool_config(&default_tool_config);
        default_config.check = args.check;

        println!("🔨 Running {} formatters...", available_formatters.len());
        let results = tool_runner
            .run_tools(available_formatters, &files, &default_config)
            .await;

        // Process results
        let mut all_results = Vec::new();
        for result in results {
            match result {
                Ok(result) => {
                    let issue_count = result.issues.len();
                    println!(
                        "  ✅ {} completed with {} issues",
                        result.tool_name, issue_count
                    );

                    // Just add the result to all_results for formatting
                    all_results.push(result);
                }
                Err(err) => {
                    println!("  ❌ Error running formatter: {}", err);
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
            println!("✨ No formatting issues found!");
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
