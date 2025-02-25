use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Main error type for Siren
#[derive(Debug, Error)]
pub enum SirenError {
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Tool execution errors
    #[error("Tool execution error: {0}")]
    Tool(#[from] ToolError),

    /// Project detection errors
    #[error("Project detection error: {0}")]
    Detection(#[from] DetectionError),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

/// Configuration related errors
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Error loading configuration file
    #[error("Failed to load config from {path}: {message}")]
    LoadError { path: PathBuf, message: String },

    /// Error parsing configuration
    #[error("Failed to parse config: {0}")]
    ParseError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// TOML parsing error
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
}

/// Tool execution errors
#[derive(Debug, Error)]
pub enum ToolError {
    /// Tool not found
    #[error("Tool '{0}' not found")]
    NotFound(String),

    /// Tool execution failed
    #[error("Failed to execute tool '{name}': {message}")]
    ExecutionFailed { name: String, message: String },

    /// Tool returned error
    #[error("Tool '{name}' failed with exit code {code}: {message}")]
    ToolFailed {
        name: String,
        code: i32,
        message: String,
    },

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

/// Project detection errors
#[derive(Debug, Error)]
pub enum DetectionError {
    /// Invalid directory
    #[error("The path '{0}' is not a valid directory or file\nPlease provide a valid directory path, specific file, or a glob pattern (e.g., src/*.rs)")]
    InvalidDirectory(PathBuf),

    /// Detection failed
    #[error("Detection failed: {0}")]
    DetectionFailed(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}
