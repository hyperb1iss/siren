//! Core data models for Siren

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

pub mod language;
pub mod results;
pub mod tools;

pub use language::*;
pub use tools::*;

/// Project information detected by the ProjectDetector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// Languages detected in the project
    pub languages: Vec<Language>,

    /// Frameworks detected in the project
    pub frameworks: Vec<Framework>,

    /// Number of files per language
    pub file_counts: HashMap<Language, usize>,

    /// Tools detected in the project (like existing linter configs)
    pub detected_tools: Vec<DetectedTool>,
}

/// A tool detected in the project (like an existing linter config)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedTool {
    /// Name of the detected tool
    pub name: String,

    /// Path to the configuration file
    pub config_path: PathBuf,

    /// Type of the tool
    pub tool_type: ToolType,

    /// Language the tool is for
    pub language: Language,
}

/// Result from running a lint tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    /// Name of the tool
    pub tool_name: String,

    /// Tool information
    pub tool: Option<ToolInfo>,

    /// Whether the tool ran successfully
    pub success: bool,

    /// Issues found by the tool
    pub issues: Vec<LintIssue>,

    /// Time taken to execute the tool
    pub execution_time: Duration,

    /// Standard output from the tool
    pub stdout: Option<String>,

    /// Standard error from the tool
    pub stderr: Option<String>,
}

/// A specific issue found by a linter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintIssue {
    /// Severity of the issue
    pub severity: IssueSeverity,

    /// Issue message
    pub message: String,

    /// File where the issue was found
    pub file: Option<PathBuf>,

    /// Line number
    pub line: Option<usize>,

    /// Column number
    pub column: Option<usize>,

    /// Code snippet
    pub code: Option<String>,

    /// Whether a fix is available
    pub fix_available: bool,
}

/// Severity levels for issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, PartialOrd, Ord)]
pub enum IssueSeverity {
    /// Error - must be fixed
    Error,

    /// Warning - should be fixed
    Warning,

    /// Information - might be worth looking at
    Info,

    /// Style - code style recommendation
    Style,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueSeverity::Error => write!(f, "Error"),
            IssueSeverity::Warning => write!(f, "Warning"),
            IssueSeverity::Info => write!(f, "Info"),
            IssueSeverity::Style => write!(f, "Style"),
        }
    }
}

/// Tool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Tool name
    pub name: String,

    /// Tool type
    pub tool_type: ToolType,

    /// Language this tool is for
    pub language: Language,

    /// Whether this tool is available on the system
    pub available: bool,

    /// Version of the tool
    pub version: Option<String>,

    /// Description of the tool
    pub description: String,
}
