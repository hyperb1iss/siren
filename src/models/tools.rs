use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString};

/// Types of tools
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, EnumIter,
)]
#[strum(serialize_all = "lowercase")]
pub enum ToolType {
    /// Code formatters (like rustfmt, black, prettier)
    Formatter,

    /// Linters (like clippy, pylint, eslint)
    Linter,

    /// Type checkers (like mypy, typescript)
    TypeChecker,

    /// Tools that can automatically fix issues
    Fixer,
}

/// Configuration for a specific tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Whether the tool is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Additional command-line arguments for the tool
    #[serde(default)]
    pub extra_args: Vec<String>,

    /// Environment variables to set when running the tool
    #[serde(default)]
    pub env_vars: std::collections::HashMap<String, String>,

    /// Custom executable path (if not using the one in PATH)
    #[serde(default)]
    pub executable_path: Option<String>,

    /// Severity level to report (error, warning, info, style)
    #[serde(default)]
    pub report_level: Option<String>,

    /// Whether to automatically fix issues (if tool supports it)
    #[serde(default)]
    pub auto_fix: bool,
}

fn default_enabled() -> bool {
    true
}
