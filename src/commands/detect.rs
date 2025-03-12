use std::path::{Path, PathBuf};

use crate::cli::DetectArgs;
use crate::detection::ProjectDetector;
use crate::errors::SirenError;
use crate::output::OutputFormatter;
use crate::utils::path_manager::PathManager;
use colored::*;

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

        // Create and initialize the path manager
        let mut path_manager = PathManager::new();

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

        // Prepare paths for detection
        let paths_to_detect = if all_paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            all_paths.clone()
        };

        // Collect files using PathManager
        path_manager.collect_files(&paths_to_detect, false)?;
        path_manager.organize_contexts();

        // Detect project information
        let (project_info, _) = if !patterns.is_empty() {
            self.detector.detect_with_patterns(dir, &patterns)?
        } else {
            self.detector.detect(&paths_to_detect)?
        };

        // Display detected project info
        let info_output = self.output_formatter.format_detection(&project_info);
        println!("{}", info_output);

        // Display additional information from PathManager
        println!("\n📂 Files by Language:");
        for language in &project_info.languages {
            let files = path_manager.get_files_by_language(*language);
            println!("  {} {:?}: {} files", "•".cyan(), language, files.len());
        }

        println!("\n🗂️ Project Contexts:");
        for (i, context) in path_manager.get_all_contexts().iter().enumerate() {
            let lang_str = context
                .language
                .map_or("Unknown".to_string(), |l| format!("{:?}", l));
            println!(
                "  {} Context {}: {} ({} files)",
                "•".cyan(),
                i + 1,
                lang_str,
                context.files.len()
            );
            println!("    Root: {}", context.root.display());
        }

        Ok(())
    }
}
