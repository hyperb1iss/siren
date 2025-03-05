//! Utility functions for Siren

use globset::{Glob, GlobSetBuilder};
use log::{debug, log_enabled, Level};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

// Export file selection utilities
pub mod file_selection;

/// Log a command that is about to be executed
pub fn log_command(command: &Command) {
    // Only log if debug level is enabled (which corresponds to verbose mode)
    if log_enabled!(Level::Debug) {
        // Get the program name
        let program = command.get_program().to_string_lossy();

        // Get all arguments
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();

        // Get working directory if set
        let working_dir = command
            .get_current_dir()
            .map(|p| format!(" (in {})", p.display()))
            .unwrap_or_default();

        // Format the full command
        let full_command = format!("{} {}{}", program, args.join(" "), working_dir);

        debug!("🔮 Executing: {}", full_command);
    }
}

/// Check if a command exists in PATH
pub fn command_exists<S: AsRef<OsStr>>(command: S) -> bool {
    let cmd = command.as_ref();
    debug!("Checking if command exists: {:?}", cmd);
    let command_str = cmd.to_str().unwrap_or("[non-utf8]");
    let result = which::which(command_str).is_ok();
    debug!("Command {:?} exists: {}", cmd, result);
    result
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

/// Collect files from a directory, respecting gitignore and skipping .git directory
pub fn collect_files_with_gitignore(dir: &Path) -> Result<Vec<PathBuf>, crate::errors::SirenError> {
    let mut files = Vec::new();

    // Debug: Print the directory we're scanning
    debug!("Starting directory search from: {:?}", dir);

    // Check if .gitignore exists in the directory
    let gitignore_path = dir.join(".gitignore");
    if gitignore_path.exists() && log_enabled!(Level::Debug) {
        debug!("Found .gitignore at {:?}", gitignore_path);
    }

    // Use ignore to respect gitignore rules
    let mut walker_builder = ignore::WalkBuilder::new(dir);
    walker_builder
        .hidden(false) // Don't ignore hidden files by default
        .git_ignore(true) // Respect .gitignore
        .git_global(true) // Respect global gitignore
        .git_exclude(true) // Respect .git/info/exclude
        .filter_entry(|entry| {
            let path = entry.path();

            // Skip .git directories and any path containing .git segment
            if path.to_string_lossy().contains("/.git/")
                || path.to_string_lossy().contains("\\.git\\")
                || path.file_name().is_some_and(|name| name == ".git")
            {
                if log_enabled!(Level::Trace) {
                    debug!("Skipping git-related path: {:?}", path);
                }
                return false;
            }

            true
        });

    let walker = walker_builder.build();

    // Debug counters
    let mut total_files = 0;
    let mut ignored_files = 0;

    for entry in walker.filter_map(Result::ok) {
        let path = entry.path();
        total_files += 1;

        // Skip any path that contains .git anywhere in its components
        if path.components().any(|c| {
            if let std::path::Component::Normal(name) = c {
                name.to_string_lossy() == ".git"
            } else {
                false
            }
        }) {
            ignored_files += 1;
            continue;
        }

        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            files.push(path.to_path_buf());
        }
    }

    if log_enabled!(Level::Debug) {
        debug!("Total files scanned: {}", total_files);
        debug!("Files ignored: {}", ignored_files);
        debug!("Files collected: {}", files.len());
    }

    Ok(files)
}

/// Optimize file paths by grouping them by directory when possible
/// This helps reduce command line length and improves performance for tools
/// that can process entire directories at once
pub fn optimize_paths_for_tools(files: &[PathBuf]) -> Vec<PathBuf> {
    if files.is_empty() {
        return Vec::new();
    }

    // First, find common parent directories for the files
    let mut dir_files: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

    // Group files by their immediate parent directory
    for file in files {
        if let Some(parent) = file.parent() {
            dir_files
                .entry(parent.to_path_buf())
                .or_default()
                .push(file.clone());
        } else {
            // If no parent (e.g., root files), keep them as is
            dir_files
                .entry(PathBuf::from("."))
                .or_default()
                .push(file.clone());
        }
    }

    // Find directories where all files have the same extension
    let mut result = Vec::new();
    let mut handled_files = HashSet::new();

    // Process directories with most files first (most likely to benefit from optimization)
    let mut dirs: Vec<_> = dir_files.into_iter().collect();
    dirs.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    for (dir, dir_files) in dirs {
        // Skip if all files in this directory are already handled
        if dir_files.iter().all(|f| handled_files.contains(f)) {
            continue;
        }

        // Get unhandled files in this directory
        let unhandled_files: Vec<_> = dir_files
            .iter()
            .filter(|f| !handled_files.contains(*f))
            .cloned()
            .collect();

        if unhandled_files.is_empty() {
            continue;
        }

        // Check if all files have the same extension
        let extensions: HashSet<_> = unhandled_files
            .iter()
            .filter_map(|f| f.extension())
            .collect();

        // If all files have the same extension and there are multiple files,
        // use the directory instead
        if extensions.len() <= 1 && unhandled_files.len() > 1 {
            // Only use the directory if it's within the project (not system directories)
            // This is a simple heuristic to avoid scanning unrelated directories
            let is_project_dir = dir.starts_with(std::env::current_dir().unwrap_or_default());

            if is_project_dir {
                result.push(dir);
                for file_path in &unhandled_files {
                    handled_files.insert(file_path.clone());
                }
            } else {
                // Add individual files if not a project directory
                for file_path in &unhandled_files {
                    result.push(file_path.clone());
                    handled_files.insert(file_path.clone());
                }
            }
        } else {
            // Add individual files if they have different extensions
            for file_path in &unhandled_files {
                result.push(file_path.clone());
                handled_files.insert(file_path.clone());
            }
        }
    }

    // Add any remaining unhandled files individually
    for file in files {
        if !handled_files.contains(file) {
            result.push(file.clone());
        }
    }

    result
}
