use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use strum_macros::{Display, EnumString};

/// Results from running a lint tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    /// Name of the tool that produced this result
    pub tool_name: String,

    /// Whether the tool ran successfully
    pub success: bool,

    /// Issues found by the tool
    pub issues: Vec<LintIssue>,

    /// Time it took to run the tool
    pub execution_time: Duration,

    /// Raw stdout from the tool
    pub stdout: Option<String>,

    /// Raw stderr from the tool
    pub stderr: Option<String>,
}

/// A specific issue found by a linter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintIssue {
    /// Severity of the issue
    pub severity: IssueSeverity,

    /// Error message
    pub message: String,

    /// File where the issue was found
    pub file: Option<PathBuf>,

    /// Line number (1-indexed)
    pub line: Option<usize>,

    /// Column number (1-indexed)
    pub column: Option<usize>,

    /// Error/warning code (like E501 for line too long)
    pub code: Option<String>,

    /// Whether a fix is available for this issue
    pub fix_available: bool,
}

/// Severity levels for issues
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
)]
#[strum(serialize_all = "lowercase")]
pub enum IssueSeverity {
    /// Errors that must be fixed
    Error,

    /// Warnings that should be fixed
    Warning,

    /// Informational messages
    Info,

    /// Style suggestions
    Style,
}

impl IssueSeverity {
    /// Get the emoji representation of this severity
    pub fn emoji(&self) -> &'static str {
        match self {
            IssueSeverity::Error => "❌",
            IssueSeverity::Warning => "⚠️",
            IssueSeverity::Info => "ℹ️",
            IssueSeverity::Style => "💄",
        }
    }

    /// Get the color name for this severity
    pub fn color_name(&self) -> &'static str {
        match self {
            IssueSeverity::Error => "red",
            IssueSeverity::Warning => "yellow",
            IssueSeverity::Info => "blue",
            IssueSeverity::Style => "magenta",
        }
    }
}
