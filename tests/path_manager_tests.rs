use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use siren::models::Language;
use siren::tools::LintTool;
use siren::utils::path_manager::PathManager;
use tempfile::{tempdir, TempDir};

// Mock tool for testing
struct MockPythonTool;

impl LintTool for MockPythonTool {
    fn name(&self) -> &str {
        "mock_python_tool"
    }

    fn can_handle(&self, file_path: &Path) -> bool {
        file_path.extension().is_some_and(|ext| ext == "py")
    }

    fn execute(
        &self,
        _files: &[PathBuf],
        _config: &siren::models::tools::ToolConfig,
    ) -> Result<siren::models::LintResult, siren::errors::ToolError> {
        unimplemented!()
    }

    fn tool_type(&self) -> siren::models::ToolType {
        siren::models::ToolType::Linter
    }

    fn languages(&self) -> Vec<Language> {
        vec![Language::Python]
    }

    fn description(&self) -> &str {
        "Mock Python Tool"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn version(&self) -> Option<String> {
        Some("1.0.0".to_string())
    }
}

// Mock tool for testing
struct MockRustTool;

impl LintTool for MockRustTool {
    fn name(&self) -> &str {
        "mock_rust_tool"
    }

    fn can_handle(&self, file_path: &Path) -> bool {
        file_path.extension().is_some_and(|ext| ext == "rs")
    }

    fn execute(
        &self,
        _files: &[PathBuf],
        _config: &siren::models::tools::ToolConfig,
    ) -> Result<siren::models::LintResult, siren::errors::ToolError> {
        unimplemented!()
    }

    fn tool_type(&self) -> siren::models::ToolType {
        siren::models::ToolType::Linter
    }

    fn languages(&self) -> Vec<Language> {
        vec![Language::Rust]
    }

    fn description(&self) -> &str {
        "Mock Rust Tool"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn version(&self) -> Option<String> {
        Some("1.0.0".to_string())
    }
}

fn create_test_project() -> TempDir {
    let dir = tempdir().unwrap();

    // Create a Python project
    let python_dir = dir.path().join("python_project");
    fs::create_dir_all(&python_dir).unwrap();

    // Create pyproject.toml
    let pyproject_path = python_dir.join("pyproject.toml");
    let mut pyproject_file = File::create(pyproject_path).unwrap();
    pyproject_file
        .write_all(b"[tool.poetry]\nname = \"test\"\n")
        .unwrap();

    // Create Python files
    let py_file1 = python_dir.join("main.py");
    let mut py_file1_handle = File::create(py_file1).unwrap();
    py_file1_handle
        .write_all(b"print('Hello, world!')")
        .unwrap();

    let py_file2 = python_dir.join("utils.py");
    let mut py_file2_handle = File::create(py_file2).unwrap();
    py_file2_handle
        .write_all(b"def add(a, b): return a + b")
        .unwrap();

    // Create a Rust project
    let rust_dir = dir.path().join("rust_project");
    fs::create_dir_all(&rust_dir).unwrap();

    // Create Cargo.toml
    let cargo_path = rust_dir.join("Cargo.toml");
    let mut cargo_file = File::create(cargo_path).unwrap();
    cargo_file
        .write_all(b"[package]\nname = \"test\"\nversion = \"0.1.0\"\n")
        .unwrap();

    // Create src directory
    let rust_src_dir = rust_dir.join("src");
    fs::create_dir_all(&rust_src_dir).unwrap();

    // Create Rust files
    let rs_file1 = rust_src_dir.join("main.rs");
    let mut rs_file1_handle = File::create(rs_file1).unwrap();
    rs_file1_handle
        .write_all(b"fn main() { println!(\"Hello, world!\"); }")
        .unwrap();

    let rs_file2 = rust_src_dir.join("lib.rs");
    let mut rs_file2_handle = File::create(rs_file2).unwrap();
    rs_file2_handle
        .write_all(b"pub fn add(a: i32, b: i32) -> i32 { a + b }")
        .unwrap();

    // Create a standalone file outside of any project
    let standalone_file = dir.path().join("standalone.txt");
    let mut standalone_file_handle = File::create(standalone_file).unwrap();
    standalone_file_handle
        .write_all(b"This is a standalone file")
        .unwrap();

    dir
}

#[test]
fn test_path_manager_file_collection() {
    let dir = create_test_project();

    // Create a PathManager
    let mut path_manager = PathManager::new();

    // Add all files in the test project
    let all_paths = vec![dir.path().to_path_buf()];
    path_manager.collect_files(&all_paths, false).unwrap();

    // Verify we collected all files
    let all_files = path_manager.get_all_files();
    assert!(all_files.len() >= 5); // At least 5 files (2 Python, 2 Rust, 1 standalone)

    // Verify files are grouped by language
    let python_files = path_manager.get_files_by_language(Language::Python);
    assert_eq!(python_files.len(), 2);

    let rust_files = path_manager.get_files_by_language(Language::Rust);
    assert_eq!(rust_files.len(), 2);
}

#[test]
fn test_path_manager_context_organization() {
    let dir = create_test_project();

    // Create a PathManager
    let mut path_manager = PathManager::new();

    // Add all files in the test project
    let all_paths = vec![dir.path().to_path_buf()];
    path_manager.collect_files(&all_paths, false).unwrap();

    // Organize contexts
    path_manager.organize_contexts();

    // Verify contexts are created
    let contexts = path_manager.get_all_contexts();
    assert!(contexts.len() >= 3); // At least 3 contexts (Python project, Rust project, standalone)

    // Verify Python context
    let python_context = contexts
        .iter()
        .find(|ctx| ctx.language == Some(Language::Python));
    assert!(python_context.is_some());
    assert_eq!(python_context.unwrap().files.len(), 2);

    // Verify Rust context
    let rust_context = contexts
        .iter()
        .find(|ctx| ctx.language == Some(Language::Rust));
    assert!(rust_context.is_some());
    assert_eq!(rust_context.unwrap().files.len(), 2);
}

#[test]
fn test_path_manager_tool_specific_paths() {
    let dir = create_test_project();

    // Create a PathManager
    let mut path_manager = PathManager::new();

    // Add all files in the test project
    let all_paths = vec![dir.path().to_path_buf()];
    path_manager.collect_files(&all_paths, false).unwrap();

    // Create mock tools
    let python_tool = MockPythonTool;
    let rust_tool = MockRustTool;

    // Get files for Python tool
    let python_files = path_manager.get_files_for_tool(&python_tool);
    assert_eq!(python_files.len(), 2);

    // Get files for Rust tool
    let rust_files = path_manager.get_files_for_tool(&rust_tool);
    assert_eq!(rust_files.len(), 2);

    // Verify optimized paths
    let optimized_python_files = path_manager.get_optimized_paths_for_tool(&python_tool);
    assert!(optimized_python_files.len() <= 2); // Could be optimized to just the directory

    let optimized_rust_files = path_manager.get_optimized_paths_for_tool(&rust_tool);
    assert!(optimized_rust_files.len() <= 2); // Could be optimized to just the directory
}

#[test]
fn test_path_manager_contexts_for_tool() {
    // Create a test project with Python and Rust files
    let dir = create_test_project();

    // Create a PathManager and add the files
    let mut path_manager = PathManager::new();
    path_manager.add_file(dir.path().join("test.py"));
    path_manager.add_file(dir.path().join("test.rs"));

    // Organize contexts
    path_manager.organize_contexts();

    // Create mock tools
    let python_tool = MockPythonTool;
    let rust_tool = MockRustTool;

    // Get files for Python tool
    let python_files = path_manager.get_files_for_tool(&python_tool);
    assert_eq!(python_files.len(), 1);
    assert!(python_files[0].to_string_lossy().ends_with("test.py"));

    // Get files for Rust tool
    let rust_files = path_manager.get_files_for_tool(&rust_tool);
    assert_eq!(rust_files.len(), 1);
    assert!(rust_files[0].to_string_lossy().ends_with("test.rs"));
}
