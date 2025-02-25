use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::cli::{CheckArgs, DetectArgs, FixArgs, FormatArgs, Verbosity};
use crate::config::{ConfigProvider, SirenConfig, ToolConfig as ConfigToolConfig};
use crate::detection::ProjectDetector;
use crate::errors::SirenError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::{ProjectInfo, ToolType};
use crate::output::OutputFormatter;
use crate::runner::ToolRunner;
use crate::tools::{LintTool, ToolRegistry};

/// Core application that orchestrates the workflow of Siren
pub struct SirenApp<D, C, R, O>
where
    D: ProjectDetector,
    C: ConfigProvider,
    R: ToolRegistry + Clone,
    O: OutputFormatter,
{
    detector: D,
    config_provider: C,
    tool_registry: R,
    output_formatter: O,
    verbosity: Verbosity,
}

impl<D, C, R, O> SirenApp<D, C, R, O>
where
    D: ProjectDetector,
    C: ConfigProvider,
    R: ToolRegistry + Clone,
    O: OutputFormatter,
{
    /// Create a new instance of SirenApp
    pub fn new(detector: D, config_provider: C, tool_registry: R, output_formatter: O) -> Self {
        Self {
            detector,
            config_provider,
            tool_registry,
            output_formatter,
            verbosity: Verbosity::default(),
        }
    }

    /// Set the verbosity level
    pub fn with_verbosity(mut self, verbosity: Verbosity) -> Self {
        self.verbosity = verbosity;
        self
    }

    /// Run the check command (lint)
    pub async fn check(
        &self,
        args: CheckArgs,
        paths: Vec<PathBuf>,
        git_modified_only: bool,
    ) -> Result<(), SirenError> {
        // Load configuration
        let config = self.load_config(&paths)?;

        // Detect project information
        let project_info = self.detect_project(&paths)?;

        if self.verbosity >= Verbosity::Normal {
            // Display detected project info
            let info_output = self.output_formatter.format_detection(&project_info);
            println!("{}", info_output);
        }

        // Select appropriate tools based on project_info and args
        let tools = self.select_tools_for_check(&project_info, &args, &config)?;

        // Run the selected tools
        if self.verbosity >= Verbosity::Normal {
            println!("🔮 Running checks with {} tools...", tools.len());
        }

        // Get files to check
        let files = if git_modified_only {
            let dir = paths
                .first()
                .map(|p| p.as_path())
                .unwrap_or_else(|| Path::new("."));
            crate::utils::get_git_modified_files(dir)?
        } else {
            paths
        };

        // Get default tool config
        let default_tool_config = config.tools.get("default").cloned().unwrap_or_default();

        // Create a tool runner
        let tool_runner = ToolRunner::new(self.tool_registry.clone());

        // Run each linter and collect results
        let mut all_results = Vec::new();
        for linter in tools {
            // Get tool-specific config or use default
            let config_tool_config = config
                .tools
                .get(linter.name())
                .cloned()
                .unwrap_or_else(|| default_tool_config.clone());

            // Convert to the correct ToolConfig type for the runner
            let config_for_runner = ConfigToolConfig {
                enabled: config_tool_config.enabled,
                config_file: config_tool_config.config_file.clone(),
                extra_args: config_tool_config.extra_args.clone(),
            };

            // Execute the linter
            let linter_ref = linter.as_ref();
            let results = tool_runner
                .run_tools(vec![linter_ref], &files, &config_for_runner)
                .await;

            // Process results
            for result in results {
                match result {
                    Ok(result) => all_results.push(result),
                    Err(err) => {
                        if self.verbosity >= Verbosity::Normal {
                            eprintln!("❌ Error running {}: {}", linter.name(), err);
                        }
                    }
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
        } else if self.verbosity >= Verbosity::Normal {
            println!("✨ No issues found!");
        }

        Ok(())
    }

    /// Run the format command
    pub async fn format(
        &self,
        args: FormatArgs,
        paths: Vec<PathBuf>,
        git_modified_only: bool,
    ) -> Result<(), SirenError> {
        // Load configuration
        let config = self.load_config(&paths)?;

        // Detect project information
        let project_info = self.detect_project(&paths)?;

        if self.verbosity >= Verbosity::Normal {
            // Display detected project info
            let info_output = self.output_formatter.format_detection(&project_info);
            println!("{}", info_output);
        }

        // Select appropriate formatting tools
        let mut formatters = Vec::new();
        for language in &project_info.languages {
            let language_formatters = self
                .tool_registry
                .get_tools_for_language_and_type(*language, ToolType::Formatter);
            for formatter in language_formatters {
                formatters.push(formatter);
            }
        }

        if formatters.is_empty() {
            if self.verbosity >= Verbosity::Normal {
                println!("⚠️ No formatters found for the detected languages.");
            }
            return Ok(());
        }

        // Get files to format
        let files = if git_modified_only {
            let dir = paths
                .first()
                .map(|p| p.as_path())
                .unwrap_or_else(|| Path::new("."));
            crate::utils::get_git_modified_files(dir)?
        } else {
            paths
        };

        if self.verbosity >= Verbosity::Normal {
            println!(
                "💅 Formatting {} files with {} formatters...",
                files.len(),
                formatters.len()
            );
        }

        // Default tool config
        let default_tool_config = ConfigToolConfig::default();

        // Format files with each formatter
        let mut all_results = Vec::new();
        for formatter in formatters {
            // Get tool-specific config or use default
            let config_tool_config = config
                .tools
                .get(formatter.name())
                .cloned()
                .unwrap_or_else(|| default_tool_config.clone());
            let tool_config = convert_tool_config(&config_tool_config);

            // Execute the formatter
            match formatter.execute(&files, &tool_config) {
                Ok(result) => {
                    all_results.push(result);
                }
                Err(err) => {
                    if self.verbosity >= Verbosity::Normal {
                        eprintln!("❌ Error running {}: {}", formatter.name(), err);
                    }
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
        } else if self.verbosity >= Verbosity::Normal {
            println!("✨ No formatting issues found!");
        }

        Ok(())
    }

    /// Run the fix command
    pub async fn fix(
        &self,
        args: FixArgs,
        paths: Vec<PathBuf>,
        git_modified_only: bool,
    ) -> Result<(), SirenError> {
        // Load configuration
        let config = self.load_config(&paths)?;

        // Detect project information
        let project_info = self.detect_project(&paths)?;

        if self.verbosity >= Verbosity::Normal {
            // Display detected project info
            let info_output = self.output_formatter.format_detection(&project_info);
            println!("{}", info_output);
        }

        // Select appropriate fixing tools
        let mut fixers = Vec::new();
        for language in &project_info.languages {
            let language_fixers = self
                .tool_registry
                .get_tools_for_language_and_type(*language, ToolType::Fixer);
            for fixer in language_fixers {
                fixers.push(fixer);
            }
        }

        if fixers.is_empty() {
            if self.verbosity >= Verbosity::Normal {
                println!("⚠️ No fixers found for the detected languages.");
            }
            return Ok(());
        }

        // Get files to fix
        let files = if git_modified_only {
            let dir = paths
                .first()
                .map(|p| p.as_path())
                .unwrap_or_else(|| Path::new("."));
            crate::utils::get_git_modified_files(dir)?
        } else {
            paths
        };

        if self.verbosity >= Verbosity::Normal {
            println!(
                "🧹 Fixing {} files with {} fixers...",
                files.len(),
                fixers.len()
            );
        }

        // Get default tool config
        let default_tool_config = config.tools.get("default").cloned().unwrap_or_default();

        // Execute fixers and collect results
        let mut all_results = Vec::new();
        for fixer in fixers {
            // Get tool-specific config or use default
            let config_tool_config = config
                .tools
                .get(fixer.name())
                .cloned()
                .unwrap_or_else(|| default_tool_config.clone());
            let tool_config = convert_tool_config(&config_tool_config);

            // Execute the fixer
            match fixer.execute(&files, &tool_config) {
                Ok(result) => {
                    all_results.push(result);
                }
                Err(err) => {
                    if self.verbosity >= Verbosity::Normal {
                        eprintln!("❌ Error running {}: {}", fixer.name(), err);
                    }
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
        } else if self.verbosity >= Verbosity::Normal {
            println!("✨ No issues to fix!");
        }

        Ok(())
    }

    /// Run the detect command
    pub fn detect(&self, args: DetectArgs, paths: Vec<PathBuf>) -> Result<(), SirenError> {
        // Detect project information
        let project_info = self.detect_project(&paths)?;

        // Format and display the detection results
        let output = self.output_formatter.format_detection(&project_info);
        println!("{}", output);

        // Show available tools
        if self.verbosity >= Verbosity::Verbose {
            println!("\n🧰 Available tools:");

            for language in &project_info.languages {
                let tools = self.tool_registry.get_tools_for_language(*language);
                if !tools.is_empty() {
                    println!("\n{:?}:", language);

                    // Group tools by type
                    let mut formatters = Vec::new();
                    let mut linters = Vec::new();
                    let mut type_checkers = Vec::new();
                    let mut fixers = Vec::new();

                    for tool in tools {
                        match tool.tool_type() {
                            ToolType::Formatter => formatters.push(tool),
                            ToolType::Linter => linters.push(tool),
                            ToolType::TypeChecker => type_checkers.push(tool),
                            ToolType::Fixer => fixers.push(tool),
                        }
                    }

                    if !formatters.is_empty() {
                        println!(
                            "  Formatters: {}",
                            formatters
                                .iter()
                                .map(|t| t.name())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }

                    if !linters.is_empty() {
                        println!(
                            "  Linters: {}",
                            linters
                                .iter()
                                .map(|t| t.name())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }

                    if !type_checkers.is_empty() {
                        println!(
                            "  Type Checkers: {}",
                            type_checkers
                                .iter()
                                .map(|t| t.name())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }

                    if !fixers.is_empty() {
                        println!(
                            "  Fixers: {}",
                            fixers
                                .iter()
                                .map(|t| t.name())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                }
            }
        }

        Ok(())
    }

    // Helper methods

    /// Load configuration from the provided paths
    fn load_config(&self, paths: &[PathBuf]) -> Result<SirenConfig, SirenError> {
        // Use the first path as base directory or current dir if empty
        let base_dir = paths
            .first()
            .map(|p| p.as_path())
            .unwrap_or_else(|| Path::new("."));

        self.config_provider.load_config(base_dir)
    }

    /// Detect project information from the provided paths
    fn detect_project(&self, paths: &[PathBuf]) -> Result<ProjectInfo, SirenError> {
        // Use the first path or current dir if empty
        let dir = paths
            .first()
            .map(|p| p.as_path())
            .unwrap_or_else(|| Path::new("."));

        self.detector.detect(dir)
    }

    /// Select appropriate tools for checking based on project info and arguments
    fn select_tools_for_check<'a>(
        &'a self,
        project_info: &ProjectInfo,
        args: &CheckArgs,
        config: &SirenConfig,
    ) -> Result<Vec<Arc<dyn LintTool>>, SirenError> {
        let mut selected_tools = Vec::new();

        // If specific tools are requested, use those
        if let Some(tool_names) = &args.tools {
            for name in tool_names {
                if let Some(tool) = self.tool_registry.get_tool_by_name(name) {
                    selected_tools.push(tool);
                } else if self.verbosity >= Verbosity::Normal {
                    eprintln!("⚠️ Tool '{}' not found", name);
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
        for language in &project_info.languages {
            let language_tools = self.tool_registry.get_tools_for_language(*language);

            // For general check, prefer linters and type checkers
            let filtered_tools: Vec<_> = language_tools
                .into_iter()
                .filter(|tool| {
                    let tool_type = tool.tool_type();
                    tool_type == ToolType::Linter || tool_type == ToolType::TypeChecker
                })
                .collect();

            selected_tools.extend(filtered_tools);
        }

        // If strict mode, add additional strict tools
        if args.strict {
            // TODO: Add strictness logic here
        }

        Ok(selected_tools)
    }
}

// Helper function to convert from config::ToolConfig to models::tools::ToolConfig
fn convert_tool_config(config: &ConfigToolConfig) -> ModelsToolConfig {
    ModelsToolConfig {
        enabled: config.enabled,
        extra_args: config.extra_args.clone().unwrap_or_default(),
        env_vars: std::collections::HashMap::new(),
        executable_path: config
            .config_file
            .clone()
            .map(|p| p.to_string_lossy().to_string()),
        report_level: None,
        auto_fix: false,
    }
}
