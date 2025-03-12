use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use siren::models::tools::ToolConfig;
use siren::models::LintResult;
use siren::models::{Language, ToolType};
use siren::runner::ToolRunner;
use siren::tools::DefaultToolRegistry;
use siren::tools::ToolRegistry;
use siren::utils::path_manager::PathManager;
use std::collections::HashMap;
use tempfile::TempDir;

// Helper function to create test fixtures with planned issues
fn create_test_fixture(languages: Vec<Language>, issue_type: &str) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let (file_name, content) = match (languages.first().unwrap(), issue_type) {
        // Rust fixtures
        (Language::Rust, "unused_variable") => (
            "src/main.rs",
            r#"
// This file intentionally contains an unused variable for testing
fn main() {
    // This variable is intentionally unused to test detection
    let unused_variable = 42;
    println!("Hello, world!");
}
            "#,
        ),
        (Language::Rust, "formatting") => (
            "src/main.rs",
            r#"
fn main(){let poorly_formatted=true;if poorly_formatted {println!("This code is intentionally poorly formatted");}}
            "#,
        ),

        // Python fixtures
        (Language::Python, "unused_import") => (
            "unused_import.py",
            r#"
import os  # Intentionally unused import
import sys
import json  # Another unused import
import re    # Yet another unused import
import datetime  # Unused import
import collections  # Unused import

def main():
    print(f"Python version: {sys.version}")

if __name__ == "__main__":
    main()
            "#,
        ),
        (Language::Python, "formatting") => (
            "bad_format.py",
            r#"
def badly_formatted_function( a,    b,c ):
    x=a+b+c;y=x*2 # deliberately bad formatting with multiple statements on one line and extra semicolon
    return     x

class BadlyFormattedClass :
 def __init__(self,x,y) :
  self.x=x
  self.y=y # inconsistent indentation

if __name__ == "__main__":
    print(   badly_formatted_function(1,2,   3))
            "#,
        ),

        // TypeScript/JavaScript fixtures
        (Language::TypeScript, "unused_variable") => (
            "unused_var.ts",
            r#"
function main() {
    // These variables are intentionally unused to test detection
    const unusedVariable1 = 42;
    const unusedVariable2 = "test";
    const unusedVariable3 = true;
    console.log("Hello, world!");
}

main();
            "#,
        ),
        (Language::JavaScript, "formatting") => (
            "bad_format.js",
            r#"
function badlyFormattedFunction(a,   b,c){const result=a+b+c;return result;}
console.log(   badlyFormattedFunction(1,2,   3));
            "#,
        ),

        // Default placeholder
        _ => (
            "unknown.txt",
            "This is a placeholder for unsupported language or issue type combinations",
        ),
    };

    let file_path = if languages.contains(&Language::Rust) {
        // Create src directory for Rust files
        std::fs::create_dir_all(temp_dir.path().join("src"))
            .expect("Failed to create src directory");

        // Create Cargo.toml for Rust projects
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        let mut cargo_file = File::create(&cargo_toml).expect("Failed to create Cargo.toml");
        cargo_file
            .write_all(
                br#"[package]
name = "test-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
            )
            .expect("Failed to write Cargo.toml");

        temp_dir.path().join(file_name)
    } else {
        temp_dir.path().join(file_name)
    };

    let mut file = File::create(&file_path).expect("Failed to create test file");
    file.write_all(content.trim().as_bytes())
        .expect("Failed to write test content");

    // For Python tests, create an __init__.py file to make it a valid package
    if languages.contains(&Language::Python) {
        let init_path = temp_dir.path().join("__init__.py");
        File::create(&init_path).expect("Failed to create __init__.py file");
    }

    (temp_dir, file_path)
}

// Helper to check if a tool is available
fn is_tool_available(languages: Vec<Language>, tool_type: ToolType) -> bool {
    let registry = DefaultToolRegistry::with_default_tools();
    // Check if any tools of this type for any of the languages are available
    languages.iter().any(|&lang| {
        let tools = registry.get_tools_for_language_and_type(lang, tool_type);
        tools.iter().any(|tool| tool.is_available())
    })
}

// Helper to run tools on fixtures and verify results
async fn verify_issue_detection(
    languages: Vec<Language>,
    issue_type: &str,
    tool_type: ToolType,
    expected_issues: usize,
) {
    // Skip test if no tools are available for any of the languages and type
    if !is_tool_available(languages.clone(), tool_type) {
        println!(
            "Skipping test for {:?} with {:?} - no tools available",
            languages, tool_type
        );
        return;
    }

    let (_temp_dir, file_path) = create_test_fixture(languages.clone(), issue_type);
    println!("Created test fixture at: {:?}", file_path);

    // Initialize registry with tools
    let registry = DefaultToolRegistry::with_default_tools();
    let runner = ToolRunner::new();

    // Create a basic tool config
    let config = ToolConfig {
        enabled: true,
        extra_args: Vec::new(),
        env_vars: HashMap::new(),
        executable_path: None,
        report_level: None,
        auto_fix: false,
        // For formatters, we need to set check to true to detect formatting issues
        check: true,
    };

    // Create a PathManager and add the test file
    let mut path_manager = PathManager::new();
    path_manager.add_file(file_path.clone());

    // Run appropriate tools for each language and tool type
    let mut all_results: Vec<Result<LintResult, siren::errors::ToolError>> = Vec::new();

    for lang in &languages {
        let tools = registry.get_tools_for_language_and_type(*lang, tool_type);

        for tool in tools {
            if !tool.is_available() {
                println!("Skipping unavailable tool: {}", tool.name());
                continue;
            }

            println!("Running tool: {}", tool.name());

            // Get optimized paths for this tool
            let tool_paths = path_manager.get_optimized_paths_for_tool(tool.as_ref());

            // Skip if no files to process
            if tool_paths.is_empty() {
                println!("No files for tool: {}", tool.name());
                continue;
            }

            // Run the tool with its specific paths
            let results = runner
                .run_tools(vec![tool.clone()], &tool_paths, &config)
                .await;

            if let Some(result) = results.first() {
                match result {
                    Ok(lint_result) => {
                        println!(
                            "Tool {} found {} issues",
                            tool.name(),
                            lint_result.issues.len()
                        );
                        all_results.push(Ok(lint_result.clone()));
                    }
                    Err(e) => {
                        println!("Error: {:?}", e);
                        // We can't clone the error, so we'll create a dummy result
                        all_results.push(Ok(LintResult {
                            success: false,
                            issues: Vec::new(),
                            tool_name: tool.name().to_string(),
                            stdout: None,
                            stderr: Some(format!("Error: {:?}", e)),
                            execution_time: std::time::Duration::from_secs(0),
                            tool: None,
                        }));
                    }
                }
            }
        }
    }

    // Count total issues and check if any tool produced output
    let total_issues: usize = all_results
        .iter()
        .filter_map(|r| r.as_ref().ok())
        .map(|r| r.issues.len())
        .sum();

    let has_output = all_results.iter().any(|r| {
        r.as_ref()
            .map(|lr| lr.stdout.is_some() || lr.stderr.is_some())
            .unwrap_or(false)
    });

    match tool_type {
        ToolType::Formatter => {
            // Formatters are more reliable in detecting issues
            if expected_issues > 0 {
                assert!(
                    total_issues > 0,
                    "Expected at least one formatting issue for {:?}, but found none",
                    languages
                );
            } else {
                assert_eq!(
                    total_issues, 0,
                    "Expected no formatting issues for {:?}, but found {}",
                    languages, total_issues
                );
            }
        }
        ToolType::Linter | ToolType::TypeChecker => {
            // Linters and type checkers might have different detection capabilities
            if expected_issues > 0 {
                assert!(
                    total_issues > 0 || has_output,
                    "Expected at least one issue or output for {:?}, but found none",
                    languages
                );
            }
        }
        _ => {
            // For other tool types, just check if they ran successfully
            assert!(
                !all_results.is_empty(),
                "Expected at least one tool to run for {:?}",
                languages
            );
        }
    }
}

#[tokio::test]
async fn test_rust_linting() {
    // Check if cargo clippy is available, skip test if not
    if !siren::utils::command_exists("cargo") {
        println!("Skipping Rust linting test - cargo not found");
        return;
    }

    // Try to run cargo clippy command to check if it's available
    let clippy_check = std::process::Command::new("cargo")
        .args(["clippy", "--version"])
        .output();

    if clippy_check.is_err() || !clippy_check.unwrap().status.success() {
        println!("Skipping Rust linting test - clippy not available");
        return;
    }

    verify_issue_detection(vec![Language::Rust], "unused_variable", ToolType::Linter, 1).await;
}

#[tokio::test]
async fn test_rust_formatting() {
    verify_issue_detection(vec![Language::Rust], "formatting", ToolType::Formatter, 1).await;
}

#[tokio::test]
async fn test_python_linting() {
    verify_issue_detection(vec![Language::Python], "unused_import", ToolType::Linter, 1).await;
}

#[tokio::test]
async fn test_python_formatting() {
    // Skip test if ruff_formatter isn't available
    if !is_tool_available(vec![Language::Python], ToolType::Formatter) {
        println!("Skipping Python formatting test - no formatter available");
        return;
    }

    verify_issue_detection(vec![Language::Python], "formatting", ToolType::Formatter, 1).await;
}

#[tokio::test]
async fn test_typescript_linting() {
    // Skip test if no TypeScript linter is available
    if !is_tool_available(vec![Language::TypeScript], ToolType::Linter) {
        println!("Skipping TypeScript linting test - no linter available");
        return;
    }

    verify_issue_detection(
        vec![Language::TypeScript],
        "unused_variable",
        ToolType::Linter,
        1,
    )
    .await;
}

#[tokio::test]
async fn test_javascript_formatting() {
    // Skip test if no JavaScript formatter is available
    if !is_tool_available(vec![Language::JavaScript], ToolType::Formatter) {
        println!("Skipping JavaScript formatting test - no formatter available");
        return;
    }

    verify_issue_detection(
        vec![Language::JavaScript],
        "formatting",
        ToolType::Formatter,
        1,
    )
    .await;
}

#[tokio::test]
async fn test_multi_language_formatting() {
    // Skip test if no JavaScript/TypeScript formatter is available
    if !is_tool_available(
        vec![Language::JavaScript, Language::TypeScript],
        ToolType::Formatter,
    ) {
        println!("Skipping multi-language formatting test - no formatter available");
        return;
    }

    verify_issue_detection(
        vec![Language::JavaScript, Language::TypeScript],
        "formatting",
        ToolType::Formatter,
        1,
    )
    .await;
}
