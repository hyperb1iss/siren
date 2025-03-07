use clap::Parser;
use siren::cli::{Cli, Commands, Verbosity};
use std::path::PathBuf;

#[test]
fn test_cli_default_values() {
    // Test default values when no arguments are provided
    let cli = Cli::parse_from(["siren"]);

    assert!(cli.command.is_none());
    assert!(cli.paths.is_empty());
    assert!(!cli.git_modified);
    assert_eq!(cli.language, None);
    assert_eq!(cli.fail_level, None);
    assert_eq!(cli.verbose, 0);
    assert!(!cli.quiet);
    assert_eq!(cli.config, None);
    assert!(!cli.ci);
}

#[test]
fn test_verbosity_levels() {
    // Test quiet flag
    let cli = Cli::parse_from(["siren", "--quiet"]);
    assert!(cli.quiet);

    // Test verbose flags
    let cli = Cli::parse_from(["siren", "-v"]);
    assert_eq!(cli.verbose, 1);

    let cli = Cli::parse_from(["siren", "-vv"]);
    assert_eq!(cli.verbose, 2);

    let cli = Cli::parse_from(["siren", "-vvv"]);
    assert_eq!(cli.verbose, 3);

    // Test verbosity from value
    let verbosity: Verbosity = 0.into();
    assert_eq!(verbosity, Verbosity::Quiet);

    let verbosity: Verbosity = 1.into();
    assert_eq!(verbosity, Verbosity::Normal);

    let verbosity: Verbosity = 2.into();
    assert_eq!(verbosity, Verbosity::Verbose);

    let verbosity: Verbosity = 3.into();
    assert_eq!(verbosity, Verbosity::Debug);

    // Test that values larger than 3 still result in Debug
    let verbosity: Verbosity = 10.into();
    assert_eq!(verbosity, Verbosity::Debug);
}

#[test]
fn test_check_command() {
    // Basic check command
    let cli = Cli::parse_from(["siren", "check", "src/"]);

    match cli.command {
        Some(Commands::Check(args)) => {
            assert!(!args.strict);
            assert_eq!(args.tools, None);
            assert_eq!(args.tool_types, None);
            assert_eq!(args.format, "pretty");
            assert!(!args.auto_fix);
            assert_eq!(args.paths, vec![PathBuf::from("src/")]);
        }
        _ => panic!("Expected Check command"),
    }

    // Check command with options
    let cli = Cli::parse_from([
        "siren",
        "check",
        "--strict",
        "--tools",
        "rustfmt,clippy",
        "--tool-types",
        "formatter,linter",
        "--format",
        "json",
        "--auto-fix",
        "src/",
        "tests/",
    ]);

    match cli.command {
        Some(Commands::Check(args)) => {
            assert!(args.strict);
            assert_eq!(args.tools, Some(vec!["rustfmt,clippy".to_string()]));
            assert_eq!(args.tool_types, Some(vec!["formatter,linter".to_string()]));
            assert_eq!(args.format, "json");
            assert!(args.auto_fix);
            assert_eq!(
                args.paths,
                vec![PathBuf::from("src/"), PathBuf::from("tests/")]
            );
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn test_format_command() {
    // Basic format command
    let cli = Cli::parse_from(["siren", "format", "src/"]);

    match cli.command {
        Some(Commands::Format(args)) => {
            assert!(!args.check);
            assert_eq!(args.tools, None);
            assert_eq!(args.paths, vec![PathBuf::from("src/")]);
        }
        _ => panic!("Expected Format command"),
    }

    // Format command with options
    let cli = Cli::parse_from([
        "siren",
        "format",
        "--check",
        "--tools",
        "rustfmt,black",
        "src/",
        "tests/",
    ]);

    match cli.command {
        Some(Commands::Format(args)) => {
            assert!(args.check);
            assert_eq!(args.tools, Some(vec!["rustfmt,black".to_string()]));
            assert_eq!(
                args.paths,
                vec![PathBuf::from("src/"), PathBuf::from("tests/")]
            );
        }
        _ => panic!("Expected Format command"),
    }
}

#[test]
fn test_fix_command() {
    // Basic fix command
    let cli = Cli::parse_from(["siren", "fix", "src/"]);

    match cli.command {
        Some(Commands::Fix(args)) => {
            assert!(!args.unsafe_fixes);
            assert_eq!(args.tools, None);
            assert!(args.format); // Default is true
            assert_eq!(args.paths, vec![PathBuf::from("src/")]);
        }
        _ => panic!("Expected Fix command"),
    }

    // Fix command with options
    let cli = Cli::parse_from([
        "siren",
        "fix",
        "--unsafe-fixes",
        "--tools",
        "clippy,ruff",
        "--format",
        "src/",
        "tests/",
    ]);

    match cli.command {
        Some(Commands::Fix(args)) => {
            assert!(args.unsafe_fixes);
            assert_eq!(args.tools, Some(vec!["clippy,ruff".to_string()]));
            assert!(args.format);
            assert_eq!(
                args.paths,
                vec![PathBuf::from("src/"), PathBuf::from("tests/")]
            );
        }
        _ => panic!("Expected Fix command"),
    }
}

#[test]
fn test_list_tools_command() {
    // Basic list-tools command
    let cli = Cli::parse_from(["siren", "list-tools"]);

    match cli.command {
        Some(Commands::ListTools(args)) => {
            assert_eq!(args.language, None);
            assert_eq!(args.type_filter, None);
            assert!(!args.available);
            assert_eq!(args.format, "pretty");
        }
        _ => panic!("Expected ListTools command"),
    }

    // List-tools command with options
    let cli = Cli::parse_from([
        "siren",
        "list-tools",
        "--language",
        "rust",
        "--type-filter",
        "formatter",
        "--available",
        "--format",
        "json",
    ]);

    match cli.command {
        Some(Commands::ListTools(args)) => {
            assert_eq!(args.language, Some("rust".to_string()));
            assert_eq!(args.type_filter, Some("formatter".to_string()));
            assert!(args.available);
            assert_eq!(args.format, "json");
        }
        _ => panic!("Expected ListTools command"),
    }
}

#[test]
fn test_global_options_with_commands() {
    // Test combining global options with commands
    let cli = Cli::parse_from([
        "siren",
        "--verbose",
        "--language",
        "rust",
        "--git-modified",
        "check",
        "src/",
    ]);

    assert_eq!(cli.verbose, 1);
    assert_eq!(cli.language, Some("rust".to_string()));
    assert!(cli.git_modified);

    match cli.command {
        Some(Commands::Check(args)) => {
            assert_eq!(args.paths, vec![PathBuf::from("src/")]);
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn test_command_aliases() {
    // Test the "lint" alias for "check"
    let cli = Cli::parse_from(["siren", "lint", "src/"]);

    match cli.command {
        Some(Commands::Check(args)) => {
            assert_eq!(args.paths, vec![PathBuf::from("src/")]);
        }
        _ => panic!("Expected Check command from 'lint' alias"),
    }

    // Test the "fmt" alias for "format"
    let cli = Cli::parse_from(["siren", "fmt", "src/"]);

    match cli.command {
        Some(Commands::Format(args)) => {
            assert_eq!(args.paths, vec![PathBuf::from("src/")]);
        }
        _ => panic!("Expected Format command from 'fmt' alias"),
    }

    // Test the "tools" alias for "list-tools"
    let cli = Cli::parse_from(["siren", "tools"]);

    match cli.command {
        Some(Commands::ListTools(_)) => {}
        _ => panic!("Expected ListTools command from 'tools' alias"),
    }
}
