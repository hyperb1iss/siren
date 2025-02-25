//! Utility functions for Siren

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Check if a command exists in PATH
pub fn command_exists<S: AsRef<OsStr>>(command: S) -> bool {
    which::which(command).is_ok()
}

/// Check if a directory is a git repository
pub fn is_git_repo(dir: &Path) -> bool {
    let git_dir = dir.join(".git");
    git_dir.exists() && git_dir.is_dir()
}

/// Get list of files modified in git
pub fn get_git_modified_files(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    if !is_git_repo(dir) {
        return Ok(Vec::new());
    }

    // Run git command to get modified files
    let output = Command::new("git")
        .arg("ls-files")
        .arg("--modified")
        .arg("--others")
        .arg("--exclude-standard")
        .current_dir(dir)
        .output()?;

    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    // Parse output into file paths
    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| dir.join(line))
        .collect();

    Ok(files)
}

/// Filter files by extension
pub fn filter_files_by_extension(files: &[PathBuf], extensions: &[&str]) -> Vec<PathBuf> {
    files
        .iter()
        .filter(|path| {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                extensions.contains(&ext_str.as_str())
            } else {
                false
            }
        })
        .cloned()
        .collect()
}

/// Convert absolute path to relative path
pub fn to_relative_path(path: &Path, base_dir: &Path) -> PathBuf {
    pathdiff::diff_paths(path, base_dir).unwrap_or_else(|| path.to_path_buf())
}

/// Find a file in a directory or its parents
pub fn find_file(dir: &Path, filename: &str) -> Option<PathBuf> {
    let mut current = Some(dir);

    while let Some(dir) = current {
        let file_path = dir.join(filename);

        if file_path.exists() && file_path.is_file() {
            return Some(file_path);
        }

        current = dir.parent();
    }

    None
}

/// Pluralize a word based on count
pub fn pluralize(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{} {}", count, singular)
    } else {
        format!("{} {}", count, plural)
    }
}
