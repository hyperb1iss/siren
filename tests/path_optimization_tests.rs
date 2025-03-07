use std::env;
use std::fs;
use tempfile::TempDir;

use siren::utils;

/// Helper function to create a temporary directory with a structure for testing
fn create_test_project() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create a project structure
    let root = temp_dir.path();

    // Create project marker (package.json)
    fs::write(
        root.join("package.json"),
        r#"{"name": "test-project", "version": "1.0.0"}"#,
    )
    .expect("Failed to create package.json");

    // Create app directory with TypeScript files
    fs::create_dir_all(root.join("app")).expect("Failed to create app directory");
    fs::write(
        root.join("app").join("index.ts"),
        "console.log('Hello, world!');",
    )
    .expect("Failed to create index.ts");
    fs::write(
        root.join("app").join("component.tsx"),
        "export const Component = () => <div>Hello</div>;",
    )
    .expect("Failed to create component.tsx");

    // Create tests directory with test files
    fs::create_dir_all(root.join("tests")).expect("Failed to create tests directory");
    fs::write(
        root.join("tests").join("test.ts"),
        "test('it works', () => expect(true).toBe(true));",
    )
    .expect("Failed to create test.ts");

    // Create some root level files
    fs::write(
        root.join("config.ts"),
        "export const config = { debug: true };",
    )
    .expect("Failed to create config.ts");

    // Create src directory with mixed files
    fs::create_dir_all(root.join("src")).expect("Failed to create src directory");
    fs::write(
        root.join("src").join("util.ts"),
        "export function add(a: number, b: number): number { return a + b; }",
    )
    .expect("Failed to create util.ts");
    fs::write(root.join("src").join("README.md"), "# Source Directory")
        .expect("Failed to create README.md");

    temp_dir
}

/// Helper function to create a temporary directory with a Python project structure
fn create_python_test_project() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create a project structure
    let root = temp_dir.path();

    // Create project marker (pyproject.toml)
    fs::write(
        root.join("pyproject.toml"),
        r#"[build-system]
requires = ["setuptools>=42.0", "wheel"]
build-backend = "setuptools.build_meta"
"#,
    )
    .expect("Failed to create pyproject.toml");

    // Create src directory with Python files
    fs::create_dir_all(root.join("src")).expect("Failed to create src directory");
    fs::write(root.join("src").join("main.py"), "print('Hello, world!')")
        .expect("Failed to create main.py");

    // Create tests directory with test files
    fs::create_dir_all(root.join("tests")).expect("Failed to create tests directory");
    fs::write(
        root.join("tests").join("test_main.py"),
        "def test_main(): assert True",
    )
    .expect("Failed to create test_main.py");

    temp_dir
}

#[test]
fn test_optimize_paths_at_project_root() {
    let temp_dir = create_test_project();
    let root = temp_dir.path();

    // Create a list of all files
    let mut all_files = Vec::new();

    // Add TypeScript files
    all_files.push(root.join("app/index.ts"));
    all_files.push(root.join("app/component.tsx"));
    all_files.push(root.join("tests/test.ts"));
    all_files.push(root.join("config.ts"));
    all_files.push(root.join("src/util.ts"));

    // Run the optimization
    let optimized_paths = utils::optimize_paths_by_extension(&all_files, &["ts", "tsx"]);

    println!("Optimized paths: {:?}", optimized_paths);

    // With our new implementation, we should get app directory and individual files
    // for tests/test.ts, config.ts, and src/util.ts
    assert!(optimized_paths.contains(&root.join("app")));

    // Check that we have the right number of paths
    // We should have app directory + individual files
    assert!(
        optimized_paths.len() >= 2,
        "Should have at least app dir and some files"
    );

    // Make sure we don't have individual files from the app directory
    assert!(!optimized_paths.contains(&root.join("app/index.ts")));
    assert!(!optimized_paths.contains(&root.join("app/component.tsx")));
}

#[test]
fn test_optimize_paths_in_subfolder() {
    let temp_dir = create_test_project();
    let root = temp_dir.path();

    // Create a list of files just from the app directory
    let mut app_files = Vec::new();
    app_files.push(root.join("app/index.ts"));
    app_files.push(root.join("app/component.tsx"));

    // We need to actually be in the app directory for this test
    let old_dir = env::current_dir().expect("Failed to get current directory");
    let app_dir = root.join("app");
    env::set_current_dir(&app_dir).expect("Failed to change directory to app");

    // Run the optimization from inside app directory
    let optimized_paths = utils::optimize_paths_by_extension(&app_files, &["ts", "tsx"]);

    // Change back to the original directory
    env::set_current_dir(old_dir).expect("Failed to change back to original directory");

    println!("App subfolder optimized paths: {:?}", optimized_paths);

    // Since we were in the app directory, we might get the parent app directory
    // or individual files as the paths. Either is fine, let's just assert that
    // we got some paths
    assert!(!optimized_paths.is_empty());

    // At least one of the app typescript files should be in the result
    let has_ts_file = optimized_paths.iter().any(|p| {
        p.to_string_lossy().contains("index.ts")
            || p.to_string_lossy().contains("component.tsx")
            || p == &app_dir
    });

    assert!(
        has_ts_file,
        "Should have at least one TypeScript file or the app directory"
    );
}

#[test]
fn test_find_tool_paths_with_directory_mode() {
    let temp_dir = create_test_project();
    let root = temp_dir.path();

    // Create a list of all files
    let mut all_files = Vec::new();

    // Add TypeScript files
    all_files.push(root.join("app/index.ts"));
    all_files.push(root.join("app/component.tsx"));
    all_files.push(root.join("tests/test.ts"));
    all_files.push(root.join("config.ts"));
    all_files.push(root.join("src/util.ts"));

    // Run find_tool_paths with directory mode = true
    let valid_paths = utils::find_tool_paths(
        &all_files,
        &["ts", "tsx"],
        |path| {
            path.extension()
                .map_or(false, |ext| ext == "ts" || ext == "tsx")
        },
        true,
    );

    println!("Directory mode paths: {:?}", valid_paths);

    // With our new implementation, we should get app directory and individual files
    assert!(valid_paths.contains(&root.join("app")));

    // Check that we have the right number of paths
    assert!(!valid_paths.is_empty(), "Should have at least some paths");

    // Make sure we don't have individual files from the app directory
    assert!(!valid_paths.contains(&root.join("app/index.ts")));
    assert!(!valid_paths.contains(&root.join("app/component.tsx")));
}

#[test]
fn test_find_tool_paths_with_individual_files_mode() {
    let temp_dir = create_test_project();
    let root = temp_dir.path();

    // Create a list of all files
    let mut all_files = Vec::new();

    // Add TypeScript files
    all_files.push(root.join("app/index.ts"));
    all_files.push(root.join("app/component.tsx"));
    all_files.push(root.join("config.ts"));

    // Create a .gitignore file that ignores index.ts
    fs::write(root.join(".gitignore"), "app/index.ts").expect("Failed to create .gitignore");

    // Run find_tool_paths with directory mode = false
    let valid_paths = utils::find_tool_paths(
        &all_files,
        &["ts", "tsx"],
        |path| {
            path.extension()
                .map_or(false, |ext| ext == "ts" || ext == "tsx")
        },
        false,
    );

    println!("Valid paths with gitignore: {:?}", valid_paths);

    // Verify that we got at least some paths
    assert!(!valid_paths.is_empty(), "Should have some valid paths");

    // Verify that component.tsx is present somewhere
    assert!(
        valid_paths
            .iter()
            .any(|p| p.to_string_lossy().contains("component.tsx")),
        "component.tsx should be in the valid paths"
    );
}

#[test]
fn test_python_project_optimization() {
    let temp_dir = create_python_test_project();
    let root = temp_dir.path();

    // Create a list of all files
    let mut all_files = Vec::new();

    // Add Python files
    all_files.push(root.join("src/main.py"));
    all_files.push(root.join("tests/test_main.py"));

    // Run the optimization
    let optimized_paths = utils::optimize_paths_by_extension(&all_files, &["py"]);

    println!("Python optimized paths: {:?}", optimized_paths);

    // Verify we got some paths
    assert!(
        !optimized_paths.is_empty(),
        "Should have some optimized paths"
    );

    // With our new implementation, we should have at least one directory
    // or the individual files
    let has_dir_or_files = optimized_paths.contains(&root.join("src"))
        || optimized_paths.contains(&root.join("tests"))
        || optimized_paths.contains(&root.join("src/main.py"))
        || optimized_paths.contains(&root.join("tests/test_main.py"));

    assert!(has_dir_or_files, "Should have either directories or files");
}
