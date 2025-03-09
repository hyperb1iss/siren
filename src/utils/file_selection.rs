//! File selection utilities for Siren commands

use std::path::{Path, PathBuf};

use crate::tools::LintTool;

/// Collect files to check/fix/format based on provided paths and git modified flag
pub fn collect_files_to_process(
    paths: &[PathBuf],
    git_modified_only: bool,
) -> Result<Vec<PathBuf>, crate::errors::SirenError> {
    // If git_modified_only is true, just get modified files without any filtering
    if git_modified_only {
        // Use the current directory or the first path's parent if available
        let git_root = if !paths.is_empty() {
            if paths[0].is_dir() {
                paths[0].clone()
            } else {
                paths[0].parent().unwrap_or(Path::new(".")).to_path_buf()
            }
        } else {
            PathBuf::from(".")
        };

        return crate::utils::get_git_modified_files(&git_root)
            .map_err(crate::errors::SirenError::Io);
    }

    // If no paths provided, handle special case
    if paths.is_empty() {
        // Find project markers in current directory
        let project_markers = [
            "pyproject.toml",
            "Cargo.toml",
            "package.json",
            "composer.json",
            "go.mod",
        ];

        let current_dir = std::env::current_dir()?;

        // Check for project markers
        let has_project_marker = project_markers
            .iter()
            .any(|marker| current_dir.join(marker).exists());

        if has_project_marker {
            // Find immediate directories with processable files
            let entries = std::fs::read_dir(&current_dir)?;
            let mut immediate_dirs = Vec::new();

            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir()
                    && !path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .starts_with('.')
                {
                    // Check if directory has any non-hidden files
                    if let Ok(dir_entries) = std::fs::read_dir(&path) {
                        if dir_entries
                            .filter_map(Result::ok)
                            .any(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
                        {
                            immediate_dirs.push(path);
                        }
                    }
                }
            }

            return Ok(immediate_dirs);
        } else {
            // No project markers, just use current directory
            return Ok(vec![PathBuf::from(".")]);
        }
    }

    // For paths provided, return them directly - no expansion needed
    Ok(paths.to_vec())
}

/// Filter files to only include those that can be handled by the specified tool
pub fn filter_files_for_tool<T>(files: &[PathBuf], tool: &T) -> Vec<PathBuf>
where
    T: LintTool + ?Sized,
{
    files
        .iter()
        .filter(|file| tool.can_handle(file))
        .cloned()
        .collect()
}
