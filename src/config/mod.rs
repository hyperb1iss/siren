//! Configuration management for Siren

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::errors::{ConfigError, SirenError};
use crate::models::Language;

/// Configuration provider trait
pub trait ConfigProvider {
    /// Load configuration from the given directory
    fn load_config(&self, base_dir: &Path) -> Result<SirenConfig, SirenError>;
}

/// General configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Level at which to fail (error, warning, info, style)
    pub fail_level: String,

    /// Use relative paths in output
    pub use_relative_paths: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            fail_level: "error".to_string(),
            use_relative_paths: true,
        }
    }
}

/// Style configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleConfig {
    /// Theme to use
    pub theme: String,

    /// Use emoji in output
    pub use_emoji: bool,
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            use_emoji: true,
        }
    }
}

/// Language-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    /// Line length
    pub line_length: Option<usize>,

    /// Rules to ignore
    pub ignore_rules: Option<Vec<String>>,

    /// Additional rules to enable
    pub enable_rules: Option<Vec<String>>,
}

impl Default for LanguageConfig {
    fn default() -> Self {
        Self {
            line_length: None,
            ignore_rules: None,
            enable_rules: None,
        }
    }
}

/// Tool-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Whether the tool is enabled
    pub enabled: bool,

    /// Extra arguments to pass to the tool
    pub extra_args: Option<Vec<String>>,

    /// Configuration file to use
    pub config_file: Option<PathBuf>,

    /// Whether to automatically fix issues
    pub auto_fix: Option<bool>,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            extra_args: None,
            config_file: None,
            auto_fix: None,
        }
    }
}

/// Output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Show line numbers
    pub show_line_numbers: bool,

    /// Show file paths
    pub show_file_paths: bool,

    /// Max issues to show per category
    pub max_issues_per_category: usize,

    /// Show code snippets
    pub show_code_snippets: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            show_line_numbers: true,
            show_file_paths: true,
            max_issues_per_category: usize::MAX,
            show_code_snippets: true,
        }
    }
}

/// Main configuration for Siren
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SirenConfig {
    /// General configuration
    #[serde(default)]
    pub general: GeneralConfig,

    /// Style configuration
    #[serde(default)]
    pub style: StyleConfig,

    /// Language-specific configuration
    #[serde(default)]
    pub languages: HashMap<Language, LanguageConfig>,

    /// Tool-specific configuration
    #[serde(default)]
    pub tools: HashMap<String, ToolConfig>,

    /// Output configuration
    #[serde(default)]
    pub output: OutputConfig,
}

impl Default for SirenConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            style: StyleConfig::default(),
            languages: HashMap::new(),
            tools: HashMap::new(),
            output: OutputConfig::default(),
        }
    }
}

/// TOML configuration provider
pub struct TomlConfigProvider;

impl TomlConfigProvider {
    /// Create a new TOML configuration provider
    pub fn new() -> Self {
        Self
    }
}

impl ConfigProvider for TomlConfigProvider {
    fn load_config(&self, base_dir: &Path) -> Result<SirenConfig, SirenError> {
        // Look for .siren.toml in the given directory and parents
        let mut current_dir = Some(base_dir);

        while let Some(dir) = current_dir {
            let config_path = dir.join(".siren.toml");

            if config_path.exists() {
                // Found a config file, try to load it
                match std::fs::read_to_string(&config_path) {
                    Ok(content) => match toml::from_str::<SirenConfig>(&content) {
                        Ok(config) => return Ok(config),
                        Err(err) => {
                            return Err(ConfigError::ParseError(err.to_string()).into());
                        }
                    },
                    Err(err) => {
                        return Err(ConfigError::LoadError {
                            path: config_path,
                            message: err.to_string(),
                        }
                        .into());
                    }
                }
            }

            // Move up to parent directory
            current_dir = dir.parent();
        }

        // No config found, return defaults
        Ok(SirenConfig::default())
    }
}
