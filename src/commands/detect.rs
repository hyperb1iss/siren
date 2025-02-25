use std::path::{Path, PathBuf};

use crate::cli::DetectArgs;
use crate::detection::ProjectDetector;
use crate::errors::SirenError;
use crate::output::OutputFormatter;

/// Command handler for the detect command
pub struct DetectCommand<D, O>
where
    D: ProjectDetector + Clone,
    O: OutputFormatter + Clone,
{
    detector: D,
    output_formatter: O,
}

impl<D, O> DetectCommand<D, O>
where
    D: ProjectDetector + Clone,
    O: OutputFormatter + Clone,
{
    /// Create a new detect command handler
    pub fn new(detector: D, output_formatter: O) -> Self {
        Self {
            detector,
            output_formatter,
        }
    }

    /// Execute the detect command
    pub fn execute(&self, args: DetectArgs, paths: Vec<PathBuf>) -> Result<(), SirenError> {
        // Clone paths from args to avoid ownership issues
        let args_paths = args.paths.clone();

        // Combine paths from the Cli struct and DetectArgs
        let all_paths = if args_paths.is_empty() {
            paths.clone()
        } else {
            args_paths
        };

        // Get project root directory
        let dir = all_paths
            .first()
            .map(|p| p.as_path())
            .unwrap_or_else(|| Path::new("."));

        // Extract path patterns (anything after the first path)
        let patterns: Vec<String> = if all_paths.len() > 1 {
            all_paths
                .iter()
                .skip(1)
                .map(|p| p.to_string_lossy().to_string())
                .collect()
        } else {
            Vec::new()
        };

        // Detect project information with patterns if provided
        let project_info = if !patterns.is_empty() {
            self.detector.detect_with_patterns(dir, &patterns)?
        } else {
            self.detector.detect(dir)?
        };

        // Display detected project info
        let info_output = self.output_formatter.format_detection(&project_info);
        println!("{}", info_output);

        Ok(())
    }
}
