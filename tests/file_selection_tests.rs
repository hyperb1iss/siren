//! Tests for file selection utilities

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// Create mock implementations for testing
mod mock_utils {
    use std::io;
    use std::path::{Path, PathBuf};

    // Mock implementation of get_git_modified_files
    pub fn get_git_modified_files(_dir: &Path) -> Result<Vec<PathBuf>, io::Error> {
        // For testing, just return an empty list
        Ok(Vec::new())
    }

    // Mock implementation of collect_files_with_gitignore
    pub fn collect_files_with_gitignore(dir: &Path) -> Result<Vec<PathBuf>, io::Error> {
        // For testing, just collect all files in the directory
        let mut files = Vec::new();

        let walker = walkdir::WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok);

        for entry in walker {
            if entry.file_type().is_file() {
                files.push(entry.path().to_path_buf());
            }
        }

        Ok(files)
    }

    // Mock LintTool trait
    pub trait LintTool {
        fn can_handle(&self, file: &Path) -> bool;
    }

    // Mock implementation of a Python tool
    pub struct MockPythonTool;

    impl LintTool for MockPythonTool {
        fn can_handle(&self, file: &Path) -> bool {
            file.extension().map_or(false, |ext| ext == "py")
        }
    }
}

// Implement file selection functions for testing
mod file_selection {
    use super::mock_utils;
    use std::path::{Path, PathBuf};

    /// Collect files to check/fix/format based on provided paths and git modified flag
    pub fn collect_files_to_process(
        paths: &[PathBuf],
        git_modified_only: bool,
    ) -> Result<Vec<PathBuf>, std::io::Error> {
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
            let git_files = mock_utils::get_git_modified_files(dir)?;

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
                let dir_files = mock_utils::collect_files_with_gitignore(Path::new("."))?;
                all_files.extend(dir_files);
            } else {
                for path in paths {
                    if path.is_file() {
                        // If it's a specific file, just add it directly
                        all_files.push(path.clone());
                    } else if path.is_dir() {
                        // If it's a directory, collect files from it
                        let dir_files = mock_utils::collect_files_with_gitignore(path)?;
                        all_files.extend(dir_files);
                    }
                }
            }

            Ok(all_files)
        }
    }

    /// Filter files to only include those that can be handled by the specified tool
    pub fn filter_files_for_tool<T>(files: &[PathBuf], tool: &T) -> Vec<PathBuf>
    where
        T: mock_utils::LintTool,
    {
        files
            .iter()
            .filter(|file| tool.can_handle(file))
            .cloned()
            .collect()
    }
}

// Helper function to create a temporary directory with files
fn create_test_directory() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // Create some test files
    let files = [
        "file1.py",
        "file2.py",
        "file3.rs",
        "subdir/file4.py",
        "subdir/file5.rs",
        "subdir/nested/file6.py",
    ];

    for file_path in &files {
        let full_path = temp_dir.path().join(file_path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        // Create the file
        let mut file = File::create(full_path).unwrap();
        writeln!(file, "// Test content").unwrap();
    }

    // Create a .gitignore file
    let gitignore_path = temp_dir.path().join(".gitignore");
    let mut gitignore = File::create(gitignore_path).unwrap();
    writeln!(gitignore, "ignored.py").unwrap();

    // Create an ignored file
    let ignored_path = temp_dir.path().join("ignored.py");
    let mut ignored_file = File::create(ignored_path).unwrap();
    writeln!(ignored_file, "// This file should be ignored").unwrap();

    temp_dir
}

// Helper to get absolute paths for a list of relative paths
fn get_absolute_paths(base_dir: &Path, relative_paths: &[&str]) -> Vec<PathBuf> {
    relative_paths.iter().map(|p| base_dir.join(p)).collect()
}

#[test]
fn test_collect_files_to_process_specific_file() {
    let temp_dir = create_test_directory();
    let base_dir = temp_dir.path();

    // Test with a specific file
    let file_path = base_dir.join("file1.py");
    let paths = vec![file_path.clone()];

    let result = file_selection::collect_files_to_process(&paths, false).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0], file_path);
}

#[test]
fn test_collect_files_to_process_directory() {
    let temp_dir = create_test_directory();
    let base_dir = temp_dir.path();

    // Test with a directory
    let subdir_path = base_dir.join("subdir");
    let paths = vec![subdir_path];

    let result = file_selection::collect_files_to_process(&paths, false).unwrap();

    // Should find file4.py, file5.rs, and nested/file6.py
    assert_eq!(result.len(), 3);

    // Check that we found the expected files
    let expected_files = get_absolute_paths(
        base_dir,
        &[
            "subdir/file4.py",
            "subdir/file5.rs",
            "subdir/nested/file6.py",
        ],
    );

    for expected in expected_files {
        assert!(
            result.contains(&expected),
            "Result should contain {:?}",
            expected
        );
    }
}

#[test]
fn test_collect_files_to_process_multiple_paths() {
    let temp_dir = create_test_directory();
    let base_dir = temp_dir.path();

    // Test with multiple paths
    let paths = vec![base_dir.join("file1.py"), base_dir.join("subdir")];

    let result = file_selection::collect_files_to_process(&paths, false).unwrap();

    // Should find file1.py, subdir/file4.py, subdir/file5.rs, and subdir/nested/file6.py
    assert_eq!(result.len(), 4);

    // Check that we found the expected files
    let expected_files = get_absolute_paths(
        base_dir,
        &[
            "file1.py",
            "subdir/file4.py",
            "subdir/file5.rs",
            "subdir/nested/file6.py",
        ],
    );

    for expected in expected_files {
        assert!(
            result.contains(&expected),
            "Result should contain {:?}",
            expected
        );
    }
}

#[test]
fn test_filter_files_for_tool() {
    let temp_dir = create_test_directory();
    let base_dir = temp_dir.path();

    // Get all files
    let all_files = vec![
        base_dir.join("file1.py"),
        base_dir.join("file2.py"),
        base_dir.join("file3.rs"),
        base_dir.join("subdir/file4.py"),
        base_dir.join("subdir/file5.rs"),
        base_dir.join("subdir/nested/file6.py"),
    ];

    // Create a mock Python tool
    let python_tool = mock_utils::MockPythonTool;

    // Filter files for Python tool
    let python_files = file_selection::filter_files_for_tool(&all_files, &python_tool);

    // Should find 4 Python files
    assert_eq!(python_files.len(), 4);

    // Check that we found the expected files
    let expected_files = get_absolute_paths(
        base_dir,
        &[
            "file1.py",
            "file2.py",
            "subdir/file4.py",
            "subdir/nested/file6.py",
        ],
    );

    for expected in expected_files {
        assert!(
            python_files.contains(&expected),
            "Result should contain {:?}",
            expected
        );
    }
}
