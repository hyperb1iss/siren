use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::cli::{CheckArgs, Verbosity};
use crate::config::SirenConfig;
use crate::detection::ProjectDetector;
use crate::errors::SirenError;
use crate::models::tools::ToolConfig;
use crate::models::{Language, ProjectInfo, ToolType};
use crate::output::OutputFormatter;
use crate::runner::ToolRunner;
use crate::tools::{LintTool, ToolRegistry};

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

        // Always display detected project info
        let info_output = self.output_formatter.format_detection(&project_info);
        println!("{}", info_output);

        // Select appropriate tools based on project_info and args
        let tools = self.select_tools_for_check(&project_info, &args, config)?;

        // Always show tools being run
        println!("🔮 Running checks with {} tools...", tools.len());

        // List the tools being used (but only if there aren't too many)
        if tools.len() <= 10 {
            for tool in &tools {
                println!(
                    "   {} {} ({:?})",
                    match tool.tool_type() {
                        ToolType::Linter => "🔍",
                        ToolType::TypeChecker => "🔎",
                        ToolType::Formatter => "🎨",
                        ToolType::Fixer => "🔧",
                    },
                    tool.name(),
                    tool.language()
                );
            }
        }

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

        // Print information about files being checked - always show this information
        println!("📂 Checking {} files...", files.len());

        // Group files by language for better display
        let files_by_language = self.group_files_by_language(&files);
        for (language, lang_files) in &files_by_language {
            println!(
                "   {} {:?}: {} files",
                match language {
                    Language::Rust => "🦀",
                    Language::Python => "🐍",
                    Language::JavaScript => "🌐",
                    Language::TypeScript => "📘",
                    _ => "📄",
                },
                language,
                lang_files.len()
            );
        }
        println!();

        // Get default tool config
        let default_tool_config = config.tools.get("default").cloned().unwrap_or_default();

        // Set auto_fix from command-line arguments
        let mut default_tool_config = default_tool_config;
        default_tool_config.auto_fix = Some(args.auto_fix);

        // Create a tool runner
        let tool_runner = ToolRunner::new();

        // Run each linter and collect results
        let mut all_results = Vec::new();
        for linter in tools {
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

            // Execute the linter
            println!("Running linter: {} on {} files", linter.name(), files.len());

            // Debug: print the first few file paths
            if !files.is_empty() {
                println!("First few files being checked:");
                for (i, file) in files.iter().take(5).enumerate() {
                    println!("  {}: {}", i + 1, file.display());
                }
                if files.len() > 5 {
                    println!("  ... plus {} more files", files.len() - 5);
                }
            }

            let results = tool_runner
                .run_tools(vec![linter.clone()], &files, &config_for_runner)
                .await;

            // Process results
            for result in results {
                match result {
                    Ok(result) => {
                        // Just add the result to all_results for formatting through the formatter
                        all_results.push(result);
                    }
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
            // Print direct count of issues found for debugging
            let total_issues: usize = all_results.iter().map(|r| r.issues.len()).sum();
            println!(
                "\nFound {} issues across {} tools",
                total_issues,
                all_results.len()
            );

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
    fn group_files_by_language(&self, files: &[PathBuf]) -> HashMap<Language, Vec<PathBuf>> {
        let mut groups: HashMap<Language, Vec<PathBuf>> = HashMap::new();
        for file in files {
            let language = crate::utils::detect_language(file);
            groups.entry(language).or_default().push(file.clone());
        }
        groups
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
