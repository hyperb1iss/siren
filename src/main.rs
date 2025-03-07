mod app;
mod cli;
mod commands;
mod config;
mod detection;
mod errors;
mod models;
mod output;
mod runner;
mod tools;
mod utils;

use clap::Parser;
use cli::{Cli, Commands, FixArgs, FormatArgs, Verbosity};
use colored::Colorize;
use errors::{DetectionError, SirenError, ToolError};
use log::{debug, info, LevelFilter};
use std::path::PathBuf;
use tools::ToolRegistry;

#[tokio::main]
async fn main() -> Result<(), SirenError> {
    // This variable should be flagged by clippy as unused
    #[allow(unused_variables)]
    let _unused_var = "This is unused";

    // Parse command line arguments
    let cli = Cli::parse();

    // Convert verbosity flag
    let verbosity = if cli.quiet {
        Verbosity::Quiet
    } else {
        Verbosity::from(cli.verbose)
    };

    // Configure and initialize logger based on verbosity
    if let Err(e) = setup_logger(verbosity) {
        // If we can't set up logging, just print an error and continue
        eprintln!("Failed to initialize logger: {}", e);
    }

    debug!("Logger initialized with verbosity: {:?}", verbosity);

    // Print a welcome message only in normal verbosity mode
    if verbosity == Verbosity::Normal {
        print_stylish_welcome();
    }

    // Create the core components
    let detector = detection::DefaultProjectDetector::new();
    let config_provider = config::TomlConfigProvider::new();
    let tool_registry = tools::DefaultToolRegistry::with_default_tools();

    // Debug print all tools to help diagnose issues
    if verbosity >= Verbosity::Verbose {
        debug!("All tools registered:");
        for tool in tool_registry.get_all_tools() {
            debug!(
                "DEBUG:   - {} ({:?}) - Available: {}",
                tool.name(),
                tool.languages(),
                tool.is_available()
            );
        }
    }

    let output_formatter = output::PrettyFormatter::new();

    // Create the Siren app
    let app = app::SirenApp::new(
        detector,
        config_provider,
        tool_registry.clone(),
        output_formatter,
    )
    .with_verbosity(verbosity);

    // Get the base directory (current dir or first arg)
    let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Expand any glob patterns in the paths
    let expanded_paths = utils::expand_glob_patterns(&base_dir, &cli.paths);

    if verbosity >= Verbosity::Verbose && expanded_paths.len() > cli.paths.len() {
        println!(
            "Expanded {} glob patterns to {} paths",
            cli.paths.len(),
            expanded_paths.len()
        );
    }

    // Determine which command to run
    match cli.command.unwrap_or(Commands::Check(cli::CheckArgs {
        strict: false,
        tools: None,
        tool_types: None,
        format: "pretty".to_string(),
        auto_fix: false,
        paths: Vec::new(),
    })) {
        Commands::Check(mut args) => {
            // Also expand any glob patterns in command-specific paths
            let cmd_expanded_paths = utils::expand_glob_patterns(&base_dir, &args.paths);
            args.paths = cmd_expanded_paths;

            // Only print debug info if verbosity is high enough
            if verbosity >= Verbosity::Verbose {
                // Tools already logged at debug level above, no need to duplicate here
            }

            if let Err(e) = app.check(args, expanded_paths, cli.git_modified).await {
                print_friendly_error(&e, verbosity);
                std::process::exit(1);
            }
        }
        Commands::Format(mut format_args) => {
            // Also expand any glob patterns in command-specific paths
            let cmd_expanded_paths = utils::expand_glob_patterns(&base_dir, &format_args.paths);
            format_args.paths = cmd_expanded_paths;

            // Create a copy of format_args for the function call
            let args_copy = cli::FormatArgs {
                check: format_args.check,
                tools: format_args.tools.clone(),
                paths: format_args.paths.clone(),
            };

            if let Err(e) = app
                .format(args_copy, expanded_paths, cli.git_modified)
                .await
            {
                print_friendly_error(&e, verbosity);
                std::process::exit(1);
            }
        }
        Commands::Fix(mut fix_args) => {
            // Also expand any glob patterns in command-specific paths
            let cmd_expanded_paths = utils::expand_glob_patterns(&base_dir, &fix_args.paths);
            fix_args.paths = cmd_expanded_paths;

            // Create a copy of fix_args for the function call
            let args_copy = cli::FixArgs {
                unsafe_fixes: fix_args.unsafe_fixes,
                tools: fix_args.tools.clone(),
                format: fix_args.format,
                paths: fix_args.paths.clone(),
            };

            if let Err(e) = app.fix(args_copy, expanded_paths, cli.git_modified).await {
                print_friendly_error(&e, verbosity);
                std::process::exit(1);
            }
        }
        Commands::FormatAndFix(mut format_and_fix_args) => {
            // Also expand any glob patterns in command-specific paths
            let cmd_expanded_paths =
                utils::expand_glob_patterns(&base_dir, &format_and_fix_args.paths);
            format_and_fix_args.paths = cmd_expanded_paths.clone();

            // First run format
            let format_args = FormatArgs {
                check: format_and_fix_args.check_format,
                tools: format_and_fix_args.tools.clone(),
                paths: format_and_fix_args.paths.clone(),
            };

            if let Err(e) = app
                .format(format_args, expanded_paths.clone(), cli.git_modified)
                .await
            {
                print_friendly_error(&e, verbosity);
                std::process::exit(1);
            }

            // Then run fix
            let fix_args = FixArgs {
                unsafe_fixes: format_and_fix_args.unsafe_fixes,
                tools: format_and_fix_args.tools,
                // Don't format again since we just did it
                format: false,
                paths: format_and_fix_args.paths,
            };

            if let Err(e) = app.fix(fix_args, expanded_paths, cli.git_modified).await {
                print_friendly_error(&e, verbosity);
                std::process::exit(1);
            }
        }
        Commands::Detect(mut detect_args) => {
            // Also expand any glob patterns in command-specific paths
            let cmd_expanded_paths = utils::expand_glob_patterns(&base_dir, &detect_args.paths);
            detect_args.paths = cmd_expanded_paths;

            if let Err(e) = app.detect(detect_args, expanded_paths) {
                print_friendly_error(&e, verbosity);
                std::process::exit(1);
            }
        }
        Commands::Init(args) => {
            println!("🚀 Init command with team={}", args.team);
            // TODO: Implement init
        }
        Commands::ListTools(args) => {
            // Use the app to get the filtered tools
            let filtered_tools = if let Some(lang_str) = &args.language {
                // Parse the language name
                let lang = match lang_str.to_lowercase().as_str() {
                    "rust" => Some(models::Language::Rust),
                    "python" => Some(models::Language::Python),
                    "javascript" => Some(models::Language::JavaScript),
                    "typescript" => Some(models::Language::TypeScript),
                    "markdown" => Some(models::Language::Markdown),
                    "toml" => Some(models::Language::Toml),
                    _ => None,
                };

                if let Some(language) = lang {
                    // Filter by language
                    if let Some(type_filter) = &args.type_filter {
                        // Filter by both language and type
                        let tool_type = match type_filter.to_lowercase().as_str() {
                            "formatter" => Some(models::ToolType::Formatter),
                            "linter" => Some(models::ToolType::Linter),
                            "typechecker" => Some(models::ToolType::TypeChecker),
                            "fixer" => Some(models::ToolType::Fixer),
                            _ => None,
                        };

                        if let Some(tool_type) = tool_type {
                            tool_registry
                                .get_tools_for_language_and_type(language, tool_type)
                                .into_iter()
                                .filter(|t| !args.available || t.is_available())
                                .map(|t| models::ToolInfo {
                                    name: t.name().to_string(),
                                    tool_type: t.tool_type(),
                                    languages: t.languages(),
                                    available: t.is_available(),
                                    version: t.version(),
                                    description: t.description().to_string(),
                                })
                                .collect::<Vec<_>>()
                        } else {
                            tool_registry
                                .get_tools_for_language(language)
                                .into_iter()
                                .filter(|t| !args.available || t.is_available())
                                .map(|t| models::ToolInfo {
                                    name: t.name().to_string(),
                                    tool_type: t.tool_type(),
                                    languages: t.languages(),
                                    available: t.is_available(),
                                    version: t.version(),
                                    description: t.description().to_string(),
                                })
                                .collect::<Vec<_>>()
                        }
                    } else {
                        tool_registry
                            .get_tools_for_language(language)
                            .into_iter()
                            .filter(|t| !args.available || t.is_available())
                            .map(|t| models::ToolInfo {
                                name: t.name().to_string(),
                                tool_type: t.tool_type(),
                                languages: t.languages(),
                                available: t.is_available(),
                                version: t.version(),
                                description: t.description().to_string(),
                            })
                            .collect::<Vec<_>>()
                    }
                } else {
                    Vec::new()
                }
            } else if let Some(type_filter) = &args.type_filter {
                // Filter by type only
                let tool_type = match type_filter.to_lowercase().as_str() {
                    "formatter" => Some(models::ToolType::Formatter),
                    "linter" => Some(models::ToolType::Linter),
                    "typechecker" => Some(models::ToolType::TypeChecker),
                    "fixer" => Some(models::ToolType::Fixer),
                    _ => None,
                };

                if let Some(tool_type) = tool_type {
                    tool_registry
                        .get_tools_by_type(tool_type)
                        .into_iter()
                        .filter(|t| !args.available || t.is_available())
                        .map(|t| models::ToolInfo {
                            name: t.name().to_string(),
                            tool_type: t.tool_type(),
                            languages: t.languages(),
                            available: t.is_available(),
                            version: t.version(),
                            description: t.description().to_string(),
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                }
            } else {
                // No filters, get all tools
                tool_registry
                    .get_all_tools()
                    .into_iter()
                    .filter(|t| !args.available || t.is_available())
                    .map(|t| models::ToolInfo {
                        name: t.name().to_string(),
                        tool_type: t.tool_type(),
                        languages: t.languages(),
                        available: t.is_available(),
                        version: t.version(),
                        description: t.description().to_string(),
                    })
                    .collect::<Vec<_>>()
            };

            // Handle different output formats
            match args.format.as_str() {
                "json" => {
                    // Sort tools by language and then by name for consistent output
                    let mut sorted_tools = filtered_tools;
                    sorted_tools.sort_by(|a, b| {
                        // First sort by language
                        let a_lang = format!("{:?}", a.languages);
                        let b_lang = format!("{:?}", b.languages);

                        // Then by name - use Ord implementation directly to avoid reference issues
                        a_lang.cmp(&b_lang).then_with(|| Ord::cmp(&a.name, &b.name))
                    });

                    // Create a serializable representation
                    let json_tools: Vec<serde_json::Value> = sorted_tools
                        .iter()
                        .map(|tool| {
                            serde_json::json!({
                                "name": tool.name,
                                "tool_type": format!("{:?}", tool.tool_type),
                                "languages": format!("{:?}", tool.languages),
                                "available": tool.available,
                                "version": tool.version,
                                "description": tool.description
                            })
                        })
                        .collect();

                    // Output as JSON
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json_tools)
                            .unwrap_or_else(|_| "[]".to_string())
                    );
                }
                _ => {
                    // Pretty format (default)
                    // Print the header
                    if verbosity != Verbosity::Quiet {
                        if filtered_tools.is_empty() {
                            println!("🧰 No tools found matching your criteria");
                            return Ok(());
                        }

                        println!("🧰 Available tools in Siren:");
                    }

                    // Convert filtered_tools to tools by language map
                    let mut by_language = std::collections::HashMap::new();

                    for tool in filtered_tools {
                        // Use first language as key or "Unknown" if no languages
                        let lang_key = if !tool.languages.is_empty() {
                            format!("{:?}", tool.languages)
                        } else {
                            "Unknown".to_string()
                        };

                        by_language
                            .entry(lang_key)
                            .or_insert_with(Vec::new)
                            .push(tool);
                    }

                    // Sort languages alphabetically for consistent output
                    let mut languages: Vec<_> = by_language.keys().collect();
                    languages.sort();

                    // Output tools grouped by language
                    for language in &languages {
                        let tools = &by_language[language.as_str()];

                        println!("\n📦 {}:", language);

                        // Sort tools by name for consistent output (alphabetically)
                        let mut sorted_tools = tools.clone();
                        sorted_tools.sort_by(|a, b| a.name.cmp(&b.name));

                        // Group by tool type
                        let mut tool_types = std::collections::HashMap::new();

                        // Group tools by their type
                        for tool in sorted_tools {
                            tool_types
                                .entry(tool.tool_type)
                                .or_insert_with(Vec::new)
                                .push(tool);
                        }

                        // Sort tool types alphabetically
                        let mut types: Vec<_> = tool_types.keys().collect();
                        types.sort_by_key(|t| format!("{:?}", t));

                        // Display tools by type
                        for &tool_type in &types {
                            println!("  🔧 {:?}s:", tool_type);

                            // Get tools for this type (already sorted by name)
                            let type_tools = &tool_types[tool_type];

                            for tool in type_tools {
                                // Format availability and version information
                                let available = if tool.available {
                                    "✓".to_string()
                                } else {
                                    "✗".to_string()
                                };

                                let version = tool
                                    .version
                                    .clone()
                                    .map_or("".to_string(), |v| format!(" ({})", v));

                                // Print tool information
                                println!(
                                    "    • {} [{}{}] - {}",
                                    tool.name, available, version, tool.description
                                );
                            }
                        }
                    }
                }
            }
        }
        Commands::Report(args) => {
            println!("📊 Report command with format={}", args.format);
            // TODO: Implement report
        }
        Commands::Suggest(args) => {
            println!(
                "💫 Suggest command with max_suggestions={}",
                args.max_suggestions
            );
            // TODO: Implement suggest
        }
    }

    Ok(())
}

/// Print a stylish welcome message
fn print_stylish_welcome() {
    use output::terminal::{divider, highlight_style};

    // Don't clear the screen

    println!("{}", output::terminal::section_header("💫 Siren 💫"));
    println!("{}", highlight_style().apply_to("Ready to scan your code"));
    println!("{}", divider());
}

/// Configure the logger based on verbosity
fn setup_logger(verbosity: Verbosity) -> Result<(), fern::InitError> {
    // Determine the log level based on verbosity
    let log_level = match verbosity {
        Verbosity::Quiet => LevelFilter::Error,
        Verbosity::Normal => LevelFilter::Info,
        Verbosity::Verbose => LevelFilter::Debug,
        Verbosity::Debug => LevelFilter::Trace,
    };

    // Only show debug messages from our crate unless in debug mode
    let debug_mode = verbosity >= Verbosity::Verbose;

    // Create a custom fern logger environment that restricts debug messages
    // from external crates by default and properly filters by level
    let mut logger = fern::Dispatch::new()
        .format(move |out, message, record| {
            // Skip debug messages entirely unless in verbose/debug mode
            if (record.level() <= log::Level::Debug && !debug_mode) ||
               // Filter out debug messages from external crates in verbose mode
               (record.level() <= log::Level::Debug &&
                !record.target().starts_with("siren") &&
                debug_mode &&
                record.level() != log::Level::Error)
            {
                return;
            }

            out.finish(format_args!(
                "{}{} {}",
                chrono::Local::now().format("[%H:%M:%S]"),
                match record.level() {
                    log::Level::Error => "ERROR".bright_red(),
                    log::Level::Warn => "WARN ".yellow(),
                    log::Level::Info => "INFO ".bright_blue(),
                    log::Level::Debug => "DEBUG".bright_cyan(),
                    log::Level::Trace => "TRACE".magenta(),
                },
                message
            ))
        })
        .level(LevelFilter::Warn) // Set a higher default threshold
        .level_for("siren", log_level); // But allow our crate to use the requested level

    // Only add terminal output if not quiet
    if verbosity != Verbosity::Quiet {
        logger = logger.chain(std::io::stdout());
    }

    logger.apply()?;

    if debug_mode {
        info!("Logger initialized with level: {:?}", log_level);
    }

    Ok(())
}

/// Print a user-friendly error message
fn print_friendly_error(error: &SirenError, verbosity: Verbosity) {
    match error {
        SirenError::Detection(detection_err) => {
            let title = "Detection Error";
            let message = match detection_err {
                DetectionError::InvalidDirectory(path) => format!(
                    "The path '{}' is not a valid directory or file",
                    path.display()
                ),
                DetectionError::DetectionFailed(msg) => format!("Detection failed: {}", msg),
                DetectionError::Io(err) => format!("File system error: {}", err),
            };

            // Get detailed help message
            let details = match detection_err {
                DetectionError::InvalidDirectory(_) =>
                    "Please provide a valid directory path, specific file, or a glob pattern (e.g., src/*.rs)",
                _ => "",
            };

            let details_option = if !details.is_empty() {
                Some(details)
            } else {
                None
            };
            output::terminal::error_panel(title, &message, details_option);
        }
        SirenError::Tool(tool_err) => {
            let title = "Tool Error";
            let message = match tool_err {
                ToolError::NotFound(name) => format!("Tool '{}' not found", name),
                ToolError::ExecutionFailed { name, message: _ } => {
                    format!("Failed to execute tool '{}'", name)
                }
                ToolError::ToolFailed {
                    name,
                    code,
                    message: _,
                } => format!("Tool '{}' failed with exit code {}", name, code),
                ToolError::Io(err) => format!("I/O error when running tool: {}", err),
            };

            // Determine what details to show
            let details = if matches!(tool_err, ToolError::NotFound(_)) {
                "Please make sure the tool is installed and available in your PATH"
            } else if verbosity >= Verbosity::Verbose {
                match tool_err {
                    ToolError::ExecutionFailed { message, .. } => message,
                    ToolError::ToolFailed { message, .. } => message,
                    _ => "",
                }
            } else {
                ""
            };

            let details_option = if !details.is_empty() {
                Some(details)
            } else {
                None
            };
            output::terminal::error_panel(title, &message, details_option);
        }
        SirenError::Io(io_err) => {
            let title = "I/O Error";
            let message = format!("An input/output error occurred: {}", io_err);

            // For I/O errors, we'll just show a simplified message
            output::terminal::error_panel(title, &message, None);
        }
        SirenError::Config(config_err) => {
            let title = "Configuration Error";
            let message = config_err.to_string();

            // For config errors, we don't have a good way to provide details
            // that work well with lifetimes in this context
            output::terminal::error_panel(title, &message, None);
        }
    }
}
