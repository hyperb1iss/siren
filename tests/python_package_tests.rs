//! Tests for Python package path optimization

use std::fs::{self, File};
use std::path::PathBuf;
use tempfile::TempDir;

use siren::utils::path_manager::PathManager;

/// Create a test directory structure with Python files and __init__.py files
fn create_python_project_structure() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let base_dir = temp_dir.path();

    // Create a Python project structure
    // /
    // ├── core/
    // │   ├── __init__.py
    // │   ├── models/
    // │   │   ├── __init__.py
    // │   │   ├── user.py
    // │   │   └── product.py
    // │   ├── views/
    // │   │   ├── __init__.py
    // │   │   └── home.py
    // │   └── utils.py
    // ├── tests/
    // │   ├── __init__.py
    // │   └── test_models.py
    // └── app.py

    // Create directories
    fs::create_dir_all(base_dir.join("core/models")).unwrap();
    fs::create_dir_all(base_dir.join("core/views")).unwrap();
    fs::create_dir_all(base_dir.join("tests")).unwrap();

    // Create __init__.py files
    File::create(base_dir.join("core/__init__.py")).unwrap();
    File::create(base_dir.join("core/models/__init__.py")).unwrap();
    File::create(base_dir.join("core/views/__init__.py")).unwrap();
    File::create(base_dir.join("tests/__init__.py")).unwrap();

    // Create Python files
    File::create(base_dir.join("core/models/user.py")).unwrap();
    File::create(base_dir.join("core/models/product.py")).unwrap();
    File::create(base_dir.join("core/views/home.py")).unwrap();
    File::create(base_dir.join("core/utils.py")).unwrap();
    File::create(base_dir.join("tests/test_models.py")).unwrap();
    File::create(base_dir.join("app.py")).unwrap();

    temp_dir
}

#[test]
fn test_optimize_paths_for_python_packages() {
    let temp_dir = create_python_project_structure();
    let base_dir = temp_dir.path();

    // Create a list of all Python files in the project
    let all_files = vec![
        base_dir.join("core/models/user.py"),
        base_dir.join("core/models/product.py"),
        base_dir.join("core/views/home.py"),
        base_dir.join("core/utils.py"),
        base_dir.join("tests/test_models.py"),
        base_dir.join("app.py"),
    ];

    // Create a PathManager and add files
    let mut path_manager = PathManager::new();
    path_manager.add_files(all_files.clone());

    // Organize contexts to detect Python packages
    path_manager.organize_contexts();

    // Get all contexts
    let contexts = path_manager.get_all_contexts();

    // Verify that we have at least one context for Python files
    assert!(!contexts.is_empty());

    // Verify that all files are included in the contexts
    let context_files: Vec<PathBuf> = contexts.iter().flat_map(|ctx| ctx.files.clone()).collect();

    assert_eq!(context_files.len(), all_files.len());

    // Verify that each file is included
    for file in &all_files {
        assert!(context_files.contains(file));
    }
}

#[test]
fn test_optimize_paths_with_mixed_structure() {
    let temp_dir = create_python_project_structure();
    let base_dir = temp_dir.path();

    // Create a list of mixed files (Python and non-Python)
    let mixed_files = vec![
        base_dir.join("core/models/user.py"),
        base_dir.join("core/models/product.py"),
        base_dir.join("core/views/home.py"),
        base_dir.join("core/utils.py"),
        base_dir.join("tests/test_models.py"),
        base_dir.join("app.py"),
        // Add some non-Python files
        base_dir.join("README.md"),
        base_dir.join("config.json"),
    ];

    // Create the non-Python files
    File::create(base_dir.join("README.md")).unwrap();
    File::create(base_dir.join("config.json")).unwrap();

    // Create a PathManager and add files
    let mut path_manager = PathManager::new();
    path_manager.add_files(mixed_files.clone());

    // Organize contexts to detect Python packages
    path_manager.organize_contexts();

    // Get all contexts
    let contexts = path_manager.get_all_contexts();

    // Verify that we have at least one context
    assert!(!contexts.is_empty());

    // Verify that all files are included in the contexts
    let context_files: Vec<PathBuf> = contexts.iter().flat_map(|ctx| ctx.files.clone()).collect();

    assert_eq!(context_files.len(), mixed_files.len());

    // Verify that each file is included
    for file in &mixed_files {
        assert!(context_files.contains(file));
    }
}
