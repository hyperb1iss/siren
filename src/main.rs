mod app;
mod cli;
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
use errors::SirenError;

#[tokio::main]
async fn main() -> Result<(), SirenError> {
    // This variable should be flagged by clippy as unused
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
    let output_formatter = output::PrettyFormatter::new();

    // Create the Siren app
    let app = app::SirenApp::new(detector, config_provider, tool_registry, output_formatter)
        .with_verbosity(verbosity);

    // Determine which command to run
    match cli.command.unwrap_or(Commands::Check(cli::CheckArgs {
        strict: false,
        tools: None,
        tool_types: None,
        format: "pretty".to_string(),
        auto_fix: false,
    })) {
        Commands::Check(args) => {
            app.check(args, cli.paths, cli.git_modified).await?;
        }
        Commands::Format(args) => {
            app.format(args, cli.paths, cli.git_modified).await?;
        }
        Commands::Fix(args) => {
            app.fix(args, cli.paths, cli.git_modified).await?;
        }
        Commands::FormatAndFix(args) => {
            // First run format
            let format_args = FormatArgs {
                check: args.check_format,
                tools: args.tools.clone(),
            };
            app.format(format_args, cli.paths.clone(), cli.git_modified)
                .await?;

            // Then run fix
            let fix_args = FixArgs {
                unsafe_fixes: args.unsafe_fixes,
                tools: args.tools,
                // Don't format again since we just did it
                format: false,
            };
            app.fix(fix_args, cli.paths, cli.git_modified).await?;
        }
        Commands::Detect(args) => {
            app.detect(args, cli.paths)?;
        }
        Commands::Init(args) => {
            println!("🚀 Init command with team={}", args.team);
            // TODO: Implement init
        }
        Commands::ListTools(args) => {
            println!("🧰 ListTools command with format={}", args.format);
            // TODO: Implement list_tools
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
