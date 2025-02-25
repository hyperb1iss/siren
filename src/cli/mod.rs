//! Command-line interface for Siren

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

mod output;

/// Verbosity level for output
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum Verbosity {
    /// Quiet mode - only show errors
    Quiet = 0,

    /// Normal mode - show errors and warnings
    Normal = 1,

    /// Verbose mode - show errors, warnings, and info
    Verbose = 2,

    /// Debug mode - show everything including debug info
    Debug = 3,
}

impl Default for Verbosity {
    fn default() -> Self {
        Self::Normal
    }
}

impl From<u8> for Verbosity {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Quiet,
            1 => Self::Normal,
            2 => Self::Verbose,
            _ => Self::Debug,
        }
    }
}

/// Siren - Enchanting code quality with irresistible standards
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "🧜‍♀️ Siren - Enchanting code quality with irresistible standards",
    long_about = "Siren is a bewitching frontend for multiple linting tools that makes maintaining code quality a delightful experience. Like the mythological sirens that lured sailors with their enchanting voices, Siren entices developers with beautiful output, smart defaults, and an intuitive interface - making code quality standards impossible to resist."
)]
pub struct Cli {
    /// Command to execute
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Files or directories to check
    #[arg(name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Only check files modified in git (staged and unstaged)
    #[arg(short = 'g', long)]
    pub git_modified: bool,

    /// Filter by language
    #[arg(short = 'l', long)]
    pub language: Option<String>,

    /// Fail on issues of this level or higher
    #[arg(long)]
    pub fail_level: Option<String>,

    /// Verbosity level (-q=quiet, -v=verbose, -vv=very verbose)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Quiet mode (no output unless there are errors)
    #[arg(short, long)]
    pub quiet: bool,

    /// Custom configuration file
    #[arg(short = 'c', long)]
    pub config: Option<PathBuf>,

    /// Use a theme
    #[arg(long)]
    pub theme: Option<String>,

    /// Disable emoji in output
    #[arg(long)]
    pub no_emoji: bool,

    /// CI mode (non-interactive, machine-readable output)
    #[arg(long)]
    pub ci: bool,
}

/// Commands that Siren can execute
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Check for issues (default command)
    #[command(visible_alias = "lint")]
    Check(CheckArgs),

    /// Format code
    #[command(visible_alias = "fmt")]
    Format(FormatArgs),

    /// Auto-fix issues
    Fix(FixArgs),

    /// Format code and auto-fix issues
    #[command(visible_alias = "fmt-fix")]
    FormatAndFix(FormatAndFixArgs),

    /// Detect languages and tools in a project
    Detect(DetectArgs),

    /// Initialize Siren in a project
    Init(InitArgs),

    /// List available tools
    #[command(visible_alias = "tools")]
    ListTools(ListToolsArgs),

    /// Generate a report
    Report(ReportArgs),

    /// Output suggestion for improving code quality
    Suggest(SuggestArgs),
}

/// Arguments for the check command
#[derive(Args, Debug)]
pub struct CheckArgs {
    /// Strict mode - stricter rules
    #[arg(short, long)]
    pub strict: bool,

    /// Only check specific tools
    #[arg(long)]
    pub tools: Option<Vec<String>>,

    /// Only check specific tool types (formatter, linter, etc.)
    #[arg(long)]
    pub tool_types: Option<Vec<String>>,

    /// Output format
    #[arg(long, default_value = "pretty")]
    pub format: String,

    /// Automatically fix issues when possible
    #[arg(short, long)]
    pub auto_fix: bool,

    /// Files or directories to check
    #[arg(name = "PATH")]
    pub paths: Vec<PathBuf>,
}

/// Arguments for the format command
#[derive(Args, Debug)]
pub struct FormatArgs {
    /// Check mode - don't write files, just check if they need formatting
    #[arg(short, long)]
    pub check: bool,

    /// Only format specific tools
    #[arg(long)]
    pub tools: Option<Vec<String>>,

    /// Files or directories to format
    #[arg(name = "PATH")]
    pub paths: Vec<PathBuf>,
}

/// Arguments for the fix command
#[derive(Args, Debug)]
pub struct FixArgs {
    /// Allow potentially unsafe fixes
    #[arg(long)]
    pub unsafe_fixes: bool,

    /// Only fix specific tools
    #[arg(long)]
    pub tools: Option<Vec<String>>,

    /// Run format before fixing
    #[arg(long, default_value = "true")]
    pub format: bool,

    /// Files or directories to fix
    #[arg(name = "PATH")]
    pub paths: Vec<PathBuf>,
}

/// Arguments for the detect command
#[derive(Args, Debug)]
pub struct DetectArgs {
    /// Maximum directory depth to scan
    #[arg(long, default_value = "5")]
    pub max_depth: usize,

    /// Output format
    #[arg(long, default_value = "pretty")]
    pub format: String,

    /// Files or directories to detect tools in
    #[arg(name = "PATH")]
    pub paths: Vec<PathBuf>,
}

/// Arguments for the init command
#[derive(Args, Debug)]
pub struct InitArgs {
    /// Create a configuration file for a team
    #[arg(long)]
    pub team: bool,

    /// Force overwrite existing configuration
    #[arg(short, long)]
    pub force: bool,
}

/// Arguments for the list-tools command
#[derive(Args, Debug)]
pub struct ListToolsArgs {
    /// Filter by language
    #[arg(short, long)]
    pub language: Option<String>,

    /// Filter by tool type
    #[arg(short, long)]
    pub type_filter: Option<String>,

    /// Show only available tools
    #[arg(short, long)]
    pub available: bool,

    /// Output format
    #[arg(long, default_value = "pretty")]
    pub format: String,
}

/// Arguments for the report command
#[derive(Args, Debug)]
pub struct ReportArgs {
    /// Output format (html, json, markdown)
    #[arg(short, long, default_value = "html")]
    pub format: String,

    /// Output file (defaults to stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

/// Arguments for the suggest command
#[derive(Args, Debug)]
pub struct SuggestArgs {
    /// Maximum number of suggestions to show
    #[arg(short, long, default_value = "5")]
    pub max_suggestions: usize,
}

/// Arguments for the format-and-fix command
#[derive(Args, Debug)]
pub struct FormatAndFixArgs {
    /// Allow potentially unsafe fixes
    #[arg(long)]
    pub unsafe_fixes: bool,

    /// Only use specific tools
    #[arg(long)]
    pub tools: Option<Vec<String>>,

    /// Check mode for format - don't write files, just check if they need formatting
    #[arg(short = 'c', long)]
    pub check_format: bool,

    /// Files or directories to format and fix
    #[arg(name = "PATH")]
    pub paths: Vec<PathBuf>,
}
