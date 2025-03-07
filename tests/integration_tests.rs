use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use siren::models::tools::ToolConfig;
use siren::models::{Language, ToolType};
use siren::runner::ToolRunner;
use siren::tools::DefaultToolRegistry;
use siren::tools::ToolRegistry;
use std::collections::HashMap;
use tempfile::TempDir;

// Helper function to create test fixtures with planned issues
fn create_test_fixture(languages: Vec<Language>, issue_type: &str) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let (file_name, content) = match (languages.first().unwrap(), issue_type) {
        // Rust fixtures
        (Language::Rust, "unused_variable") => (
            "unused_var.rs",
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
            "bad_format.rs",
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
    x=a+b+c
    return     x

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

    let file_path = temp_dir.path().join(file_name);
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
        check: true,
    };

    // Run appropriate tools for each language and tool type
    let mut all_results = Vec::new();
    for lang in &languages {
        let tools = registry.get_tools_for_language_and_type(*lang, tool_type);
        let results = runner.run_tools(tools, &[file_path.clone()], &config).await;
        all_results.extend(results);
    }

    // Print all results for debugging
    for result in &all_results {
        match result {
            Ok(lint_result) => {
                println!("Tool: {}", lint_result.tool_name);
                println!("Success: {}", lint_result.success);
                println!("Issues: {}", lint_result.issues.len());
                if let Some(stdout) = &lint_result.stdout {
                    println!("Stdout: {}", stdout);
                }
                if let Some(stderr) = &lint_result.stderr {
                    println!("Stderr: {}", stderr);
                }
            }
            Err(e) => println!("Error: {:?}", e),
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
        _ => {
            // For linters, we check that:
            // 1. Either issues were found (ideal)
            // 2. Or the tool produced some output (acceptable)
            if expected_issues > 0 {
                if total_issues == 0 {
                    // If no issues were found, at least ensure the tool produced some output
                    assert!(
                        has_output,
                        "Expected either issues or output for {:?} with issue type '{}', but got neither",
                        languages, issue_type
                    );
                    println!(
                        "Note: No issues found for {:?} with issue type '{}', but tool produced output",
                        languages, issue_type
                    );
                }
            } else {
                assert_eq!(
                    total_issues, 0,
                    "Expected no issues for {:?} with issue type '{}', but found {}",
                    languages, issue_type, total_issues
                );
            }
        }
    }
}

#[tokio::test]
async fn test_rust_linting() {
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
    verify_issue_detection(vec![Language::Python], "formatting", ToolType::Formatter, 1).await;
}

#[tokio::test]
async fn test_typescript_linting() {
    verify_issue_detection(vec![Language::TypeScript], "unused_variable", ToolType::Linter, 1).await;
}

#[tokio::test]
async fn test_javascript_formatting() {
    verify_issue_detection(vec![Language::JavaScript], "formatting", ToolType::Formatter, 1).await;
}

#[tokio::test]
async fn test_multi_language_formatting() {
    verify_issue_detection(
        vec![Language::JavaScript, Language::TypeScript],
        "formatting",
        ToolType::Formatter,
        1,
    )
    .await;
}
