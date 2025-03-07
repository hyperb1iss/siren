//! Tests for Python package path optimization

use std::fs::{self, File};
use tempfile::TempDir;

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

    // Optimize paths
    let optimized_paths = siren::utils::optimize_paths_for_tools(&all_files);

    // We expect only the top-level package directories and the app.py file
    let expected_paths = vec![
        base_dir.join("core"),
        base_dir.join("tests"),
        base_dir.join("app.py"),
    ];

    // Sort both vectors for comparison
    let mut optimized_paths_sorted = optimized_paths.clone();
    optimized_paths_sorted.sort();
    let mut expected_paths_sorted = expected_paths.clone();
    expected_paths_sorted.sort();

    assert_eq!(
        optimized_paths_sorted, expected_paths_sorted,
        "Optimized paths should only include top-level package directories and individual files"
    );

    // Make sure we don't have any subdirectories in the result
    assert!(
        !optimized_paths
            .iter()
            .any(|p| p.to_string_lossy().contains("models")),
        "Subdirectory 'models' should not be in the optimized paths"
    );
    assert!(
        !optimized_paths
            .iter()
            .any(|p| p.to_string_lossy().contains("views")),
        "Subdirectory 'views' should not be in the optimized paths"
    );
}

#[test]
fn test_optimize_paths_with_mixed_structure() {
    let temp_dir = create_python_project_structure();
    let base_dir = temp_dir.path();

    // Create a non-Python directory without __init__.py
    fs::create_dir_all(base_dir.join("docs")).unwrap();
    File::create(base_dir.join("docs/index.html")).unwrap();
    File::create(base_dir.join("docs/style.css")).unwrap();

    // Create a list of mixed files
    let mixed_files = vec![
        base_dir.join("core/models/user.py"),
        base_dir.join("core/views/home.py"),
        base_dir.join("docs/index.html"),
        base_dir.join("docs/style.css"),
        base_dir.join("app.py"),
    ];

    // Optimize paths
    let optimized_paths = siren::utils::optimize_paths_for_tools(&mixed_files);

    // We expect the core directory, individual HTML/CSS files, and app.py
    let expected_paths = vec![
        base_dir.join("core"),
        base_dir.join("docs/index.html"),
        base_dir.join("docs/style.css"),
        base_dir.join("app.py"),
    ];

    // Sort both vectors for comparison
    let mut optimized_paths_sorted = optimized_paths.clone();
    optimized_paths_sorted.sort();
    let mut expected_paths_sorted = expected_paths.clone();
    expected_paths_sorted.sort();

    assert_eq!(
        optimized_paths_sorted, expected_paths_sorted,
        "Optimized paths should include Python package directories and individual non-Python files"
    );
}
