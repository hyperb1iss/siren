use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::cli::{CheckArgs, DetectArgs, FixArgs, FormatArgs, Verbosity};
use crate::commands::CheckCommand;
use crate::config::{ConfigProvider, SirenConfig, ToolConfig as ConfigToolConfig};
use crate::detection::ProjectDetector;
use crate::errors::SirenError;
use crate::models::tools::ToolConfig as ModelsToolConfig;
use crate::models::{ProjectInfo, ToolType};
use crate::output::OutputFormatter;
use crate::tools::{LintTool, ToolRegistry};

/// Core application that orchestrates the workflow of Siren
pub struct SirenApp<D, C, R, O>
where
    D: ProjectDetector + Clone,
    C: ConfigProvider,
    R: ToolRegistry + Clone,
    O: OutputFormatter + Clone,
{
    detector: D,
    config_provider: C,
    tool_registry: R,
    output_formatter: O,
    verbosity: Verbosity,
}

impl<D, C, R, O> SirenApp<D, C, R, O>
where
    D: ProjectDetector + Clone,
    C: ConfigProvider,
    R: ToolRegistry + Clone,
    O: OutputFormatter + Clone,
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

        // Create a CheckCommand instance and delegate execution
        let check_command = CheckCommand::new(
            self.detector.clone(),
            self.tool_registry.clone(),
            self.output_formatter.clone(),
            self.verbosity,
        );

        // Delegate to the CheckCommand
        check_command
            .execute(args, paths, git_modified_only, &config)
            .await
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

        // Create a FormatCommand instance and delegate execution
        let format_command = crate::commands::FormatCommand::new(
            self.detector.clone(),
            self.tool_registry.clone(),
            self.output_formatter.clone(),
            self.verbosity,
        );

        // Delegate to the FormatCommand
        format_command
            .execute(args, paths, git_modified_only, &config)
            .await
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

        // Create a FixCommand instance and delegate execution
        let fix_command = crate::commands::FixCommand::new(
            self.detector.clone(),
            self.tool_registry.clone(),
            self.output_formatter.clone(),
            self.verbosity,
        );

        // Delegate to the FixCommand
        fix_command
            .execute(args, paths, git_modified_only, &config)
            .await
    }

    /// Run the detect command
    pub fn detect(&self, args: DetectArgs, paths: Vec<PathBuf>) -> Result<(), SirenError> {
        // Create a DetectCommand instance and delegate execution
        let detect_command = crate::commands::DetectCommand::new(
            self.detector.clone(),
            self.output_formatter.clone(),
            self.verbosity,
        );

        // Delegate to the DetectCommand
        detect_command.execute(args, paths)
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
            eprintln!("Detected languages: {:?}", project_info.languages);
        }

        // If specific tools are requested, use those
        if let Some(tool_names) = &args.tools {
            if self.verbosity >= Verbosity::Verbose {
                eprintln!("Specific tools requested: {:?}", tool_names);
            }
            for name in tool_names {
                if let Some(tool) = self.tool_registry.get_tool_by_name(name) {
                    if self.verbosity >= Verbosity::Verbose {
                        eprintln!("Found tool '{}', available: {}", name, tool.is_available());
                    }
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
        if self.verbosity >= Verbosity::Verbose {
            eprintln!("Selecting tools based on detected languages");
        }
        for language in &project_info.languages {
            if self.verbosity >= Verbosity::Verbose {
                eprintln!("Getting tools for language: {:?}", language);
            }
            let language_tools = self.tool_registry.get_tools_for_language(*language);
            if self.verbosity >= Verbosity::Verbose {
                eprintln!("Found {} tools for {:?}", language_tools.len(), language);
            }

            // For general check, prefer linters and type checkers
            let filtered_tools: Vec<_> = language_tools
                .into_iter()
                .filter(|tool| {
                    let tool_type = tool.tool_type();
                    let available = tool.is_available();
                    if self.verbosity >= Verbosity::Verbose {
                        eprintln!(
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
                eprintln!("Selected {} tools after filtering", filtered_tools.len());
            }
            selected_tools.extend(filtered_tools);
        }

        // If strict mode, add additional strict tools
        if args.strict {
            // TODO: Add strictness logic here
        }

        // Debug - Print selected tools only in verbose mode
        if self.verbosity >= Verbosity::Verbose {
            eprintln!("Final selected tools: {}", selected_tools.len());
            for tool in &selected_tools {
                eprintln!("  - {}, Available: {}", tool.name(), tool.is_available());
            }
        }

        Ok(selected_tools)
    }

    /// Group files by language
    fn group_files_by_language(
        &self,
        files: &[PathBuf],
    ) -> std::collections::HashMap<crate::models::Language, Vec<PathBuf>> {
        let mut groups: std::collections::HashMap<crate::models::Language, Vec<PathBuf>> =
            std::collections::HashMap::new();
        for file in files {
            let language = crate::utils::detect_language(file);
            groups.entry(language).or_default().push(file.clone());
        }
        groups
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
        auto_fix: config.auto_fix.unwrap_or(false),
        check: config.check.unwrap_or(false),
    }
}
