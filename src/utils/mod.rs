//! Utility functions for Siren

use globset::{Glob, GlobSetBuilder};
use log::{debug, log_enabled, Level};
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

/// This helps reduce command line length and improves performance for tools
/// that can process entire directories at once
pub fn optimize_paths_for_tools(files: &[PathBuf]) -> Vec<PathBuf> {
    if files.is_empty() {
        return Vec::new();
    }

    // Get the current working directory
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(_) => return files.to_vec(), // If we can't get cwd, just return the files as-is
    };

    use std::path::Path;

    debug!(
        "Starting path optimization from current directory: {:?}",
        cwd
    );

    // First, identify Python files and non-Python files
    let mut python_files = Vec::new();
    let mut non_python_files = Vec::new();

    for file in files {
        if let Some(ext) = file.extension() {
            if ext == "py" {
                python_files.push(file.clone());
            } else {
                non_python_files.push(file.clone());
            }
        } else {
            non_python_files.push(file.clone());
        }
    }

    debug!(
        "Found {} Python files and {} non-Python files",
        python_files.len(),
        non_python_files.len()
    );

    // Special handling for Python packages - find directories with __init__.py
    let mut all_python_dirs = std::collections::HashSet::new();
    let mut python_package_dirs = Vec::new();

    // Collect Python package directories (those with __init__.py)
    for file in &python_files {
        let mut current = file.clone();
        while let Some(parent) = current.parent() {
            if parent != Path::new("") && parent.starts_with(&cwd) {
                all_python_dirs.insert(parent.to_path_buf());
            }
            current = parent.to_path_buf();
        }
    }

    for dir in &all_python_dirs {
        if dir.join("__init__.py").exists() {
            python_package_dirs.push(dir.clone());
        }
    }

    debug!(
        "Found {} directories with __init__.py",
        python_package_dirs.len()
    );

    // Find top-level package directories (those that aren't subdirectories of other packages)
    let mut top_level_python_dirs = Vec::new();

    // Sort package directories by path length (shortest first)
    python_package_dirs.sort_by_key(|a| a.as_os_str().len());

    let mut handled_dirs = std::collections::HashSet::new();
    for dir in python_package_dirs {
        // Skip if this directory is already covered by a parent package
        if handled_dirs
            .iter()
            .any(|parent: &PathBuf| dir.starts_with(parent) && &dir != parent)
        {
            debug!(
                "Skipping subdirectory {:?} as it's covered by a parent package",
                dir
            );
            continue;
        }

        debug!("Adding top-level Python package directory: {:?}", dir);
        top_level_python_dirs.push(dir.clone());
        handled_dirs.insert(dir);
    }

    // For non-Python files, find top-level project directories
    let mut top_level_dirs = std::collections::HashMap::new();

    // Identify common top-level project directories for non-Python files
    for file in &non_python_files {
        if let Ok(rel_path) = file.strip_prefix(&cwd) {
            let top_level = rel_path.components().next();
            if let Some(top_dir_name) = top_level {
                let top_dir_path = cwd.join(top_dir_name.as_os_str());

                // Skip hidden directories
                let name = top_dir_name.as_os_str().to_string_lossy();
                if !name.starts_with('.') {
                    // Count files per top-level directory
                    top_level_dirs
                        .entry(top_dir_path)
                        .and_modify(|count| *count += 1)
                        .or_insert(1);
                }
            }
        }
    }

    // Combine results
    let mut result = Vec::new();
    let mut handled_files = std::collections::HashSet::new();

    // Add Python package directories first
    for dir in &top_level_python_dirs {
        result.push(dir.clone());

        // Mark Python files in these directories as handled
        for file in &python_files {
            if file.starts_with(dir) {
                handled_files.insert(file.clone());
            }
        }
    }

    // Add top-level directories for non-Python files if they have more than one file
    for (dir, count) in top_level_dirs {
        // Skip if directory is already a Python package directory
        if top_level_python_dirs.contains(&dir) {
            continue;
        }

        // Only include directories that have more than one file
        if dir.is_dir() && count > 1 {
            debug!(
                "Adding top-level directory: {:?} (contains {} files)",
                dir, count
            );
            result.push(dir.clone());

            // Mark all files in this directory as handled
            for file in files {
                if file.starts_with(&dir) {
                    handled_files.insert(file.clone());
                }
            }
        }
    }

    // Add any remaining files that weren't handled by directories
    for file in files {
        if !handled_files.contains(file) {
            debug!("Adding remaining unhandled file: {:?}", file);
            result.push(file.clone());
        }
    }

    debug!("Final optimized paths count: {}", result.len());
    result
}

/// Optimize paths for tools based on file extensions
///
/// This function optimizes the paths passed to tools by:
/// 1. Grouping files with the same extension in the same directory
/// 2. Using directory paths instead of multiple individual files when possible
/// 3. Stopping at the first directory with supported files if no specific files are given
/// 4. Respecting gitignore rules when scanning directories
///
/// `files` is the list of files to optimize
/// `extensions` is a list of file extensions to match (without the dot, e.g. ["js", "ts"])
pub fn optimize_paths_by_extension(files: &[PathBuf], extensions: &[&str]) -> Vec<PathBuf> {
    if files.is_empty() {
        return Vec::new();
    }

    // First, collect all files and their parent directories
    let mut all_dirs = std::collections::HashSet::new();
    let mut matched_files = Vec::new();
    let mut other_files = Vec::new();

    // Separate matching files and non-matching files
    for file in files {
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            if extensions.contains(&ext) {
                matched_files.push(file.clone());
                // Add all parent directories to the set
                let mut current = file.clone();
                while let Some(parent) = current.parent() {
                    if parent != Path::new("") {
                        all_dirs.insert(parent.to_path_buf());
                    }
                    current = parent.to_path_buf();
                }
            } else {
                other_files.push(file.clone());
            }
        } else {
            // Files without extension
            other_files.push(file.clone());
        }
    }

    debug!(
        "Found {} files with matching extensions ({:?}) and {} other files",
        matched_files.len(),
        extensions,
        other_files.len()
    );
    debug!("Collected {} possible parent directories", all_dirs.len());

    // If we have no matching files, just return the original files
    if matched_files.is_empty() {
        return files.to_vec();
    }

    // Group files by parent directory
    let mut files_by_dir = std::collections::HashMap::new();
    for file in &matched_files {
        if let Some(parent) = file.parent() {
            files_by_dir
                .entry(parent.to_path_buf())
                .or_insert_with(Vec::new)
                .push(file.clone());
        } else {
            // If no parent, keep the file as is
            files_by_dir
                .entry(PathBuf::from("."))
                .or_insert_with(Vec::new)
                .push(file.clone());
        }
    }

    // Find directories with multiple matching files
    let mut result = Vec::new();
    let mut handled_files = std::collections::HashSet::new();

    // First, handle directories with multiple files
    for (dir, dir_files) in &files_by_dir {
        if dir_files.len() > 1 {
            // Use the directory instead of individual files
            debug!("Using directory {:?} for {} files", dir, dir_files.len());
            result.push(dir.clone());

            // Mark all files in this directory as handled
            for file in dir_files {
                handled_files.insert(file.clone());
            }
        }
    }

    // Then, add any remaining files that weren't handled
    for file in &matched_files {
        if !handled_files.contains(file) {
            debug!("Adding individual file: {:?}", file);
            result.push(file.clone());
        }
    }

    debug!("Final optimized paths count: {}", result.len());
    if result.len() <= 5 {
        for path in &result {
            debug!("Optimized path: {:?}", path);
        }
    }

    // The final result
    result
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

/// Find and filter paths for a tool with proper optimization
///
/// This function combines path optimization and gitignore-aware directory expansion:
/// 1. Optimizes paths by grouping files with the same extensions
/// 2. Expands directories respecting gitignore rules
/// 3. Filters files based on the provided can_handle function
///
/// `files` is the list of files to process
/// `extensions` is a list of file extensions to match (without the dot, e.g. ["js", "ts"])
/// `can_handle` is a function that determines if a file can be handled by the tool
/// `use_directories` determines if the tool prefers directories (true) or individual files (false)
pub fn find_tool_paths<F>(
    files: &[PathBuf],
    extensions: &[&str],
    can_handle: F,
    use_directories: bool,
) -> Vec<PathBuf>
where
    F: Fn(&Path) -> bool,
{
    // First optimize the paths
    let optimized_paths = optimize_paths_by_extension(files, extensions);

    // If we're using directories, just return the optimized paths
    if use_directories {
        debug!("Tool supports directory-based operation, using optimized paths directly");
        return optimized_paths;
    }

    // Otherwise, expand directories respecting gitignore for tools that need individual files
    debug!("Tool requires individual files, expanding directories");
    let mut expanded_paths = Vec::new();
    for path in &optimized_paths {
        if path.is_dir() {
            // For directories, try to collect files respecting gitignore
            match collect_files_with_gitignore(path) {
                Ok(collected_files) => {
                    // Filter to only files we can handle
                    let valid_files: Vec<_> = collected_files
                        .into_iter()
                        .filter(|f| can_handle(f))
                        .collect();

                    if !valid_files.is_empty() {
                        expanded_paths.extend(valid_files);
                    } else {
                        // If no valid files found after filtering, just use the directory
                        expanded_paths.push(path.clone());
                    }
                }
                Err(_) => {
                    // If error collecting, just use the directory as-is
                    expanded_paths.push(path.clone());
                }
            }
        } else {
            // For individual files, just add them directly if they can be handled
            if can_handle(path) {
                expanded_paths.push(path.clone());
            }
        }
    }

    debug!(
        "Found {} expanded paths after filtering",
        expanded_paths.len()
    );
    expanded_paths
}
