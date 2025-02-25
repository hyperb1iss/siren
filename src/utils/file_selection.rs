//! File selection utilities for Siren commands

use std::path::{Path, PathBuf};

/// Collect files to check/fix/format based on provided paths and git modified flag
pub fn collect_files_to_process(
    paths: &[PathBuf],
    git_modified_only: bool,
) -> Result<Vec<PathBuf>, crate::errors::SirenError> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }

    // Get project root directory (for git operations)
    let dir = paths
        .first()
        .map(|p| p.as_path())
        .unwrap_or_else(|| Path::new("."));

    // Collect files to process
    if git_modified_only {
        // Get files modified in git
        let git_files = crate::utils::get_git_modified_files(dir)?;
        
        // Filter git files to only include those that match our paths
        if paths.len() == 1 && paths[0] == PathBuf::from(".") {
            // If only the current directory is specified, use all git files
            Ok(git_files)
        } else {
            Ok(git_files
                .into_iter()
                .filter(|file| {
                    paths.iter().any(|path| {
                        if path.is_dir() {
                            file.starts_with(path)
                        } else {
                            file == path
                        }
                    })
                })
                .collect())
        }
    } else {
        let mut all_files = Vec::new();

        // If only the current directory is specified, scan it
        if paths.len() == 1 && paths[0] == PathBuf::from(".") {
            let dir_files = crate::utils::collect_files_with_gitignore(Path::new("."))?;
            all_files.extend(dir_files);
        } else {
            for path in paths {
                if path.is_file() {
                    // If it's a specific file, just add it directly
                    all_files.push(path.clone());
                } else if path.is_dir() {
                    // If it's a directory, collect files from it
                    let dir_files = crate::utils::collect_files_with_gitignore(path)?;
                    all_files.extend(dir_files);
                }
            }
        }

        Ok(all_files)
    }
}

/// Filter files to only include those that can be handled by the specified tool
pub fn filter_files_for_tool<T>(
    files: &[PathBuf],
    tool: &T,
) -> Vec<PathBuf>
where
    T: crate::tools::LintTool,
{
    files
        .iter()
        .filter(|file| tool.can_handle(file))
        .cloned()
        .collect()
} 