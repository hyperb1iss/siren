use std::path::{Path, PathBuf};

use crate::cli::{CheckArgs, DetectArgs, FixArgs, FormatArgs, Verbosity};
use crate::commands::CheckCommand;
use crate::config::{ConfigProvider, SirenConfig};
use crate::detection::ProjectDetector;
use crate::errors::SirenError;
use crate::output::OutputFormatter;
use crate::tools::ToolRegistry;

/// Core application that orchestrates the workflow of Siren
pub struct SirenApp<D, C, R, O>
where
    D: ProjectDetector + Clone,
    C: ConfigProvider,
    R: ToolRegistry + Clone,
    O: OutputFormatter + Clone,
{
    detector: D,
    config_provider: C,
    tool_registry: R,
    output_formatter: O,
    verbosity: Verbosity,
}

impl<D, C, R, O> SirenApp<D, C, R, O>
where
    D: ProjectDetector + Clone,
    C: ConfigProvider,
    R: ToolRegistry + Clone,
    O: OutputFormatter + Clone,
{
    /// Create a new instance of SirenApp
    pub fn new(detector: D, config_provider: C, tool_registry: R, output_formatter: O) -> Self {
        Self {
            detector,
            config_provider,
            tool_registry,
            output_formatter,
            verbosity: Verbosity::default(),
        }
    }

    /// Set the verbosity level
    pub fn with_verbosity(mut self, verbosity: Verbosity) -> Self {
        self.verbosity = verbosity;
        self
    }

    /// Run the check command (lint)
    pub async fn check(
        &self,
        args: CheckArgs,
        paths: Vec<PathBuf>,
        git_modified_only: bool,
    ) -> Result<(), SirenError> {
        // Load configuration
        let config = self.load_config(&paths)?;

        // Create a CheckCommand instance and delegate execution
        let check_command = CheckCommand::new(
            self.detector.clone(),
            self.tool_registry.clone(),
            self.output_formatter.clone(),
            self.verbosity,
        );

        // Delegate to the CheckCommand
        check_command
            .execute(args, paths, git_modified_only, &config)
            .await
    }

    /// Run the format command
    pub async fn format(
        &self,
        args: FormatArgs,
        paths: Vec<PathBuf>,
        git_modified_only: bool,
    ) -> Result<(), SirenError> {
        // Load configuration
        let config = self.load_config(&paths)?;

        // Create a FormatCommand instance and delegate execution
        let format_command = crate::commands::FormatCommand::new(
            self.detector.clone(),
            self.tool_registry.clone(),
            self.output_formatter.clone(),
        );

        // Delegate to the FormatCommand
        format_command
            .execute(args, paths, git_modified_only, &config)
            .await
    }

    /// Run the fix command
    pub async fn fix(
        &self,
        args: FixArgs,
        paths: Vec<PathBuf>,
        git_modified_only: bool,
    ) -> Result<(), SirenError> {
        // Load configuration
        let config = self.load_config(&paths)?;

        // Create a FixCommand instance and delegate execution
        let fix_command = crate::commands::FixCommand::new(
            self.detector.clone(),
            self.tool_registry.clone(),
            self.output_formatter.clone(),
        );

        // Delegate to the FixCommand
        fix_command
            .execute(args, paths, git_modified_only, &config)
            .await
    }

    /// Run the detect command
    pub fn detect(&self, args: DetectArgs, paths: Vec<PathBuf>) -> Result<(), SirenError> {
        // Create a DetectCommand instance and delegate execution
        let detect_command = crate::commands::DetectCommand::new(
            self.detector.clone(),
            self.output_formatter.clone(),
        );

        // Delegate to the DetectCommand
        detect_command.execute(args, paths)
    }

    // Helper methods

    /// Load configuration from the provided paths
    fn load_config(&self, paths: &[PathBuf]) -> Result<SirenConfig, SirenError> {
        // Use the first path as base directory or current dir if empty
        let base_dir = paths
            .first()
            .map(|p| p.as_path())
            .unwrap_or_else(|| Path::new("."));

        self.config_provider.load_config(base_dir)
    }
}
