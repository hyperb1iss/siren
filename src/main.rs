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
use errors::{DetectionError, SirenError, ToolError};
use std::path::PathBuf;
use tools::ToolRegistry;

#[tokio::main]
async fn main() -> Result<(), SirenError> {
    // This variable should be flagged by clippy as unused
    #[allow(unused_variables)]
    let _unused_var = "This is unused";

    // Initialize logger
    env_logger::init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Convert verbosity flag
    let verbosity = if cli.quiet {
        Verbosity::Quiet
    } else {
        Verbosity::from(cli.verbose)
    };

    // Print a welcome message if not in quiet mode
    if verbosity != Verbosity::Quiet {
        println!("🧜‍♀️ Siren - Enchanting code quality with irresistible standards");
    }

    // Create the core components
    let detector = detection::DefaultProjectDetector::new();
    let config_provider = config::TomlConfigProvider::new();
    let tool_registry = tools::DefaultToolRegistry::with_default_tools();

    // Debug print all tools to help diagnose issues
    if verbosity >= Verbosity::Verbose {
        eprintln!("DEBUG: All tools registered in main.rs:");
        for tool in tool_registry.get_all_tools() {
            eprintln!(
                "DEBUG:   - {} ({:?}) - Available: {}",
                tool.name(),
                tool.language(),
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
                // Print all tools to help diagnose issues
                eprintln!("All tools registered:");
                for tool in tool_registry.get_all_tools() {
                    eprintln!(
                        "  - {} ({:?}) - Available: {}",
                        tool.name(),
                        tool.language(),
                        tool.is_available()
                    );
                }
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
                                    language: t.language(),
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
                                    language: t.language(),
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
                                language: t.language(),
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
                            language: t.language(),
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
                        language: t.language(),
                        available: t.is_available(),
                        version: t.version(),
                        description: t.description().to_string(),
                    })
                    .collect::<Vec<_>>()
            };

            // Handle different output formats
            match args.format.as_str() {
                "json" => {
                    // Create a serializable representation
                    let json_tools: Vec<serde_json::Value> = filtered_tools
                        .iter()
                        .map(|tool| {
                            serde_json::json!({
                                "name": tool.name,
                                "tool_type": format!("{:?}", tool.tool_type),
                                "language": format!("{:?}", tool.language),
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

                    // Group tools by language
                    let mut by_language = std::collections::HashMap::new();

                    for tool in filtered_tools {
                        by_language
                            .entry(tool.language)
                            .or_insert_with(Vec::new)
                            .push(tool);
                    }

                    // Sort languages for consistent output
                    let mut languages: Vec<_> = by_language.keys().collect();
                    languages.sort_by_key(|l| format!("{:?}", l));

                    // Output tools grouped by language
                    for &language in &languages {
                        let tools = &by_language[language];

                        println!("\n📦 {:?}:", language);

                        // Sort tools by type then name for consistent output
                        let mut sorted_tools = tools.clone();
                        sorted_tools.sort_by(|a, b| {
                            // Convert tool types to strings for comparison
                            let a_type = format!("{:?}", a.tool_type);
                            let b_type = format!("{:?}", b.tool_type);

                            // Compare by type and then by name
                            a_type.cmp(&b_type).then_with(|| a.name.cmp(&b.name))
                        });

                        // Group by tool type
                        let mut current_type = None;

                        for tool in sorted_tools {
                            // Print tool type header if it changed
                            if current_type != Some(tool.tool_type) {
                                current_type = Some(tool.tool_type);
                                println!("  🔧 {:?}s:", tool.tool_type);
                            }

                            // Format availability and version information
                            let available = if tool.available {
                                "✓".to_string()
                            } else {
                                "✗".to_string()
                            };

                            let version =
                                tool.version.map_or("".to_string(), |v| format!(" ({})", v));

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

/// Print a more user-friendly error message
fn print_friendly_error(err: &SirenError, verbosity: Verbosity) {
    match err {
        SirenError::Detection(detection_err) => match detection_err {
            DetectionError::InvalidDirectory(path) => {
                eprintln!(
                    "❌ Error: The path '{}' is not a valid directory",
                    path.display()
                );
                eprintln!("   Please provide a valid directory path to scan for code");
            }
            DetectionError::DetectionFailed(msg) => {
                eprintln!("❌ Error: Failed to detect project settings - {}", msg);
            }
            _ => eprintln!("❌ Error: {}", detection_err),
        },
        SirenError::Tool(tool_err) => match tool_err {
            ToolError::NotFound(name) => {
                eprintln!("❌ Error: Tool '{}' not found", name);
                eprintln!("   Please make sure the tool is installed and available in your PATH");
            }
            _ => eprintln!("❌ Error: {}", tool_err),
        },
        _ => {
            // For other errors, use the standard Display implementation
            eprintln!("❌ Error: {}", err);

            // In verbose mode, print more details
            if verbosity >= Verbosity::Verbose {
                eprintln!("Error details: {:?}", err);
            }
        }
    }
}
