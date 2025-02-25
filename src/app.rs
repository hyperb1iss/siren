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

        // Always display detected project info (removed verbosity check)
        let info_output = self.output_formatter.format_detection(&project_info);
        println!("{}", info_output);

        // Select appropriate tools based on project_info and args
        let tools = self.select_tools_for_check(&project_info, &args, &config)?;

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
            self.collect_files_with_gitignore(current_dir)?
        } else {
            // Expand directories to files
            let mut all_files = Vec::new();

            for path in &all_paths {
                if path.is_dir() {
                    let dir_files = self.collect_files_with_gitignore(path)?;
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
                    crate::models::Language::Rust => "🦀",
                    crate::models::Language::Python => "🐍",
                    crate::models::Language::JavaScript => "🌐",
                    crate::models::Language::TypeScript => "📘",
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
        let tool_runner = ToolRunner::new(self.tool_registry.clone());

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

    /// Run the format command
    pub async fn format(
        &self,
        args: FormatArgs,
        paths: Vec<PathBuf>,
        git_modified_only: bool,
    ) -> Result<(), SirenError> {
        // Load configuration
        let config = self.load_config(&paths)?;

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
            self.detect_project_with_patterns(dir, &patterns)?
        } else {
            self.detect_project(&all_paths)?
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
                .tool_registry
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
            self.collect_files_with_gitignore(current_dir)?
        } else {
            // Expand directories to files
            let mut all_files = Vec::new();

            for path in &all_paths {
                if path.is_dir() {
                    let dir_files = self.collect_files_with_gitignore(path)?;
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
        let tool_runner = ToolRunner::new(self.tool_registry.clone());

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
        let mut default_config = convert_tool_config(&default_tool_config);
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

    /// Run the fix command
    pub async fn fix(
        &self,
        args: FixArgs,
        paths: Vec<PathBuf>,
        git_modified_only: bool,
    ) -> Result<(), SirenError> {
        // First run the format command if requested
        // By default, format is run as part of fix
        if args.format {
            if self.verbosity >= Verbosity::Normal {
                println!("💅 Running format before fix...");
            }

            // Create FormatArgs without the check option
            let format_args = FormatArgs {
                check: false,
                tools: args.tools.clone(),
                paths: args.paths.clone(),
            };

            // Run the format command
            self.format(format_args, paths.clone(), git_modified_only)
                .await?;
        }

        // Load configuration
        let config = self.load_config(&paths)?;

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

        // Detect project information with patterns if provided
        let project_info = if !patterns.is_empty() {
            self.detect_project_with_patterns(dir, &patterns)?
        } else {
            self.detect_project(&all_paths)?
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
            self.collect_files_with_gitignore(current_dir)?
        } else {
            // Expand directories to files
            let mut all_files = Vec::new();

            for path in &all_paths {
                if path.is_dir() {
                    let dir_files = self.collect_files_with_gitignore(path)?;
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
        let tool_runner = ToolRunner::new(self.tool_registry.clone());

        // Prepare all available fixers
        let available_fixers: Vec<_> = fixers.into_iter().filter(|f| f.is_available()).collect();

        if available_fixers.is_empty() {
            println!("⚠️ No available fixers found for the detected languages.");
            return Ok(());
        }

        // Setup default tool configuration
        let mut default_config = convert_tool_config(&default_tool_config);
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

    /// Run the detect command
    pub fn detect(&self, args: DetectArgs, paths: Vec<PathBuf>) -> Result<(), SirenError> {
        // Clone paths from args to avoid ownership issues
        let args_paths = args.paths.clone();

        // Combine paths from the Cli struct and DetectArgs
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

        // Display detected project info
        let info_output = self.output_formatter.format_detection(&project_info);
        println!("{}", info_output);

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

    /// Collect files respecting .gitignore and skipping hidden directories
    fn collect_files_with_gitignore(&self, dir: &Path) -> Result<Vec<PathBuf>, SirenError> {
        // Create an ignore builder that respects .gitignore files
        let mut builder = ignore::WalkBuilder::new(dir);
        builder.hidden(false); // Don't skip hidden files initially (needed to process .gitignore)
        builder.git_ignore(true); // Respect .gitignore files
        builder.git_global(true); // Respect global gitignore
        builder.git_exclude(true); // Respect git exclude files

        let mut files = Vec::new();

        for result in builder.build() {
            match result {
                Ok(entry) => {
                    let path = entry.path();

                    // Skip hidden directories (starting with .)
                    if path.components().any(|c| {
                        if let std::path::Component::Normal(name) = c {
                            let name_str = name.to_string_lossy();
                            name_str.starts_with(".") && name_str != ".gitignore"
                        } else {
                            false
                        }
                    }) {
                        continue;
                    }

                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        files.push(path.to_path_buf());
                    }
                }
                Err(err) => {
                    if self.verbosity >= Verbosity::Verbose {
                        eprintln!("Error walking directory: {}", err);
                    }
                }
            }
        }

        Ok(files)
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
