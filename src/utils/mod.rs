//! Utility functions for Siren

use globset::{Glob, GlobSetBuilder};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Check if a command exists in PATH
pub fn command_exists<S: AsRef<OsStr>>(command: S) -> bool {
    which::which(command).is_ok()
}

/// Check if a command is available in PATH (alias for command_exists)
pub fn is_command_available<S: AsRef<OsStr>>(command: S) -> bool {
    command_exists(command)
}

/// Get the version of a command
pub fn get_command_version<S: AsRef<OsStr>>(command: S, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;

    if output.status.success() {
        let version_output = String::from_utf8_lossy(&output.stdout).to_string();
        // Try to extract version from the first line
        let first_line = version_output.lines().next()?;
        Some(first_line.trim().to_string())
    } else {
        None
    }
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

/// Detect the language of a file based on its extension
pub fn detect_language(file_path: &Path) -> crate::models::Language {
    use crate::models::Language;

    if let Some(extension) = file_path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        match ext.as_str() {
            "rs" => Language::Rust,
            "py" | "pyi" => Language::Python,
            "js" | "jsx" | "mjs" => Language::JavaScript,
            "ts" | "tsx" => Language::TypeScript,
            "html" | "htm" => Language::Html,
            "css" => Language::Css,
            "go" => Language::Go,
            "rb" => Language::Ruby,
            "java" => Language::Java,
            "php" => Language::Php,
            "c" => Language::C,
            "cpp" | "cc" | "cxx" | "h" | "hpp" => Language::Cpp,
            "cs" => Language::CSharp,
            "swift" => Language::Swift,
            "md" | "markdown" => Language::Markdown,
            "json" => Language::Json,
            "yml" | "yaml" => Language::Yaml,
            "toml" => Language::Toml,
            _ => Language::Unknown,
        }
    } else {
        // Handle special files without extensions
        let filename = file_path
            .file_name()
            .map(|f| f.to_string_lossy().to_lowercase());
        match filename.as_deref() {
            Some("makefile") => Language::Makefile,
            Some("dockerfile") => Language::Docker,
            _ => Language::Unknown,
        }
    }
}

/// Expand glob patterns in the provided path strings
///
/// Takes strings like "src/*.rs" and expands them into actual file paths.
/// If the path doesn't contain glob patterns or doesn't match any files,
/// it will be returned as-is.
///
/// # Arguments
///
/// * `base_dir` - The base directory for relative patterns
/// * `patterns` - List of path patterns that may contain globs
///
/// # Returns
///
/// A vector of resolved path buffers
pub fn expand_glob_patterns(base_dir: &Path, patterns: &[PathBuf]) -> Vec<PathBuf> {
    if patterns.is_empty() {
        return vec![base_dir.to_path_buf()];
    }

    let mut result = Vec::new();
    let mut has_expanded = false;

    for pattern in patterns {
        let pattern_str = pattern.to_string_lossy();

        // Check if the pattern contains wildcard characters
        if pattern_str.contains('*') || pattern_str.contains('?') || pattern_str.contains('[') {
            // Create a glob pattern
            let glob_pattern = if pattern.is_absolute() {
                pattern_str.to_string()
            } else {
                // Make the pattern relative to base_dir
                let base = base_dir.to_string_lossy();
                if base.ends_with('/') || base.ends_with('\\') {
                    format!("{}{}", base, pattern_str)
                } else {
                    format!("{}/{}", base, pattern_str)
                }
            };

            // Build a globset from this pattern
            match Glob::new(&glob_pattern) {
                Ok(glob) => {
                    let mut builder = GlobSetBuilder::new();
                    builder.add(glob);

                    if let Ok(globset) = builder.build() {
                        // Collect all matching files
                        let walker = walkdir::WalkDir::new(base_dir)
                            .follow_links(false)
                            .into_iter()
                            .filter_map(Result::ok);

                        let mut found_matches = false;

                        for entry in walker {
                            let path = entry.path();
                            let relative_path = pathdiff::diff_paths(path, base_dir)
                                .unwrap_or_else(|| path.to_path_buf());

                            if let Some(path_str) = relative_path.to_str() {
                                if globset.is_match(path_str) {
                                    result.push(path.to_path_buf());
                                    found_matches = true;
                                    has_expanded = true;
                                }
                            }
                        }

                        // If no matches were found, keep the original pattern
                        if !found_matches {
                            result.push(pattern.clone());
                        }
                    } else {
                        // If we couldn't build the globset, keep the original pattern
                        result.push(pattern.clone());
                    }
                }
                Err(_) => {
                    // If we couldn't parse the glob, keep the original pattern
                    result.push(pattern.clone());
                }
            }
        } else {
            // Not a glob pattern, add as-is
            result.push(pattern.clone());
        }
    }

    // If no globs were expanded, return the original patterns
    if !has_expanded && !patterns.is_empty() {
        return patterns.to_vec();
    }

    result
}
