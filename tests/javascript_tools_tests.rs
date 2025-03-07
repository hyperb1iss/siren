//! Integration tests for JavaScript/TypeScript tools

use std::path::PathBuf;
use std::sync::Arc;

use siren::models::{Language, ToolType};
use siren::tools::javascript::{ESLint, Prettier};
use siren::tools::DefaultToolRegistry;
use siren::tools::{LintTool, ToolRegistry};

#[test]
fn test_prettier_can_handle() {
    let prettier = Prettier::new();

    // Should handle JavaScript files
    assert!(prettier.can_handle(&PathBuf::from("test.js")));
    assert!(prettier.can_handle(&PathBuf::from("test.jsx")));

    // Should handle TypeScript files
    assert!(prettier.can_handle(&PathBuf::from("test.ts")));
    assert!(prettier.can_handle(&PathBuf::from("test.tsx")));

    // Should handle other supported files
    assert!(prettier.can_handle(&PathBuf::from("test.json")));
    assert!(prettier.can_handle(&PathBuf::from("test.css")));
    assert!(prettier.can_handle(&PathBuf::from("test.html")));

    // Should not handle unsupported files
    assert!(!prettier.can_handle(&PathBuf::from("test.rs")));
    assert!(!prettier.can_handle(&PathBuf::from("test.py")));
}

#[test]
fn test_eslint_can_handle() {
    let eslint = ESLint::new();

    // Should handle JavaScript files
    assert!(eslint.can_handle(&PathBuf::from("test.js")));
    assert!(eslint.can_handle(&PathBuf::from("test.jsx")));
    assert!(eslint.can_handle(&PathBuf::from("test.mjs")));
    assert!(eslint.can_handle(&PathBuf::from("test.cjs")));

    // Should handle TypeScript files
    assert!(eslint.can_handle(&PathBuf::from("test.ts")));
    assert!(eslint.can_handle(&PathBuf::from("test.tsx")));

    // Should not handle unsupported files
    assert!(!eslint.can_handle(&PathBuf::from("test.json")));
    assert!(!eslint.can_handle(&PathBuf::from("test.css")));
    assert!(!eslint.can_handle(&PathBuf::from("test.html")));
    assert!(!eslint.can_handle(&PathBuf::from("test.rs")));
    assert!(!eslint.can_handle(&PathBuf::from("test.py")));
}

#[test]
fn test_tool_registry_with_js_tools() {
    let mut registry = DefaultToolRegistry::new();

    // Register JavaScript/TypeScript tools
    registry.register_tool(Arc::new(Prettier::new()));
    registry.register_tool(Arc::new(ESLint::new()));

    // Get tools for JavaScript
    let js_tools = registry.get_tools_for_language(Language::JavaScript);
    assert_eq!(js_tools.len(), 2); // Prettier and ESLint

    // Get tools for TypeScript
    let ts_tools = registry.get_tools_for_language(Language::TypeScript);
    assert_eq!(ts_tools.len(), 2); // Prettier and ESLint

    // Get formatters
    let formatters = registry.get_tools_by_type(ToolType::Formatter);
    assert!(formatters.iter().any(|t| t.name() == "prettier"));

    // Get linters
    let linters = registry.get_tools_by_type(ToolType::Linter);
    assert!(linters.iter().any(|t| t.name() == "eslint"));
}

// Skip this test if the tools are not installed
#[test]
#[ignore]
fn test_tool_availability() {
    let prettier = Prettier::new();
    let eslint = ESLint::new();

    // These tests will be skipped by default since they depend on the tools being installed
    // Run with `cargo test -- --ignored` to include these tests
    if prettier.is_available() {
        println!("Prettier version: {:?}", prettier.version());
    } else {
        println!("Prettier is not available");
    }

    if eslint.is_available() {
        println!("ESLint version: {:?}", eslint.version());
    } else {
        println!("ESLint is not available");
    }
}
