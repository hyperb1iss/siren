//! Utility functions for Siren

use globset::{Glob, GlobSetBuilder};
use log::{debug, log_enabled, Level};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

// Export path management utilities
pub mod path_manager;

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
    println!("Looking for git repo in: {:?}", dir);
    if !is_git_repo(dir) {
        println!("Not a git repository: {:?}", dir);
        return Ok(Vec::new());
    }
    println!("Found git repository: {:?}", dir);

    // Get modified files using git status
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        println!("Git status failed: {}", stderr);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    println!("Git status output: {:?}", stdout);

    // Parse output into file paths - include both modified and added files
    let files: Vec<PathBuf> = stdout
        .lines()
        .filter(|line| !line.is_empty()) // Skip empty lines
        .filter_map(|line| {
            println!("Processing git status line: '{}'", line);

            // Git status --porcelain format has two status characters followed by a space
            // then the file path. Skip untracked files (marked with "??")
            if line.starts_with("??") {
                println!("Skipping untracked file");
                return None;
            }

            // Extract the filename part (after the status codes and space)
            if line.len() > 3 {
                let file_path = line[3..].trim();
                println!("Extracted file path: '{}'", file_path);
                let absolute_path = dir.join(file_path);
                println!("Resolved to absolute path: {:?}", absolute_path);
                Some(absolute_path)
            } else {
                println!("Line too short to contain a valid path");
                None
            }
        })
        .collect();

    println!("Found {} modified files: {:?}", files.len(), files);
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
        // Return an empty vector instead of base_dir when no patterns provided
        // This allows PathManager to use its project detection logic
        return Vec::new();
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

/// Check if a directory is a valid Python package
///
/// A directory is considered a valid Python package if it or any of its
/// parent directories (up to the current working directory) contains an __init__.py file.
/// This helps prevent scanning Python files in non-package directories like docs or examples.
pub fn is_valid_python_package(dir: &Path) -> bool {
    // Get the current working directory
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(_) => return true, // If we can't get the current directory, assume it's valid
    };

    // Start from the given directory and check each parent up to cwd
    let mut current = dir.to_path_buf();

    // Check if the directory itself has an __init__.py
    if current.join("__init__.py").exists() {
        return true;
    }

    // Check parent directories until we reach cwd
    while let Some(parent) = current.parent() {
        // Stop if we've reached or gone beyond the current working directory
        if !parent.starts_with(&cwd) || parent == Path::new("") {
            break;
        }

        // Check for __init__.py in this parent
        if parent.join("__init__.py").exists() {
            return true;
        }

        current = parent.to_path_buf();
    }

    false
}
