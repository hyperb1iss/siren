use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use siren::models::{tools::ToolConfig as ModelsToolConfig, IssueSeverity};
use siren::tools::html::djlint::{DjLint, DjLintFormatter};
use siren::tools::LintTool;
use siren::utils;

fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let file_path = dir.path().join(name);
    let mut file = fs::File::create(&file_path).unwrap();
    write!(file, "{}", content).unwrap();
    file_path
}

fn create_test_config() -> ModelsToolConfig {
    ModelsToolConfig {
        enabled: true,
        extra_args: vec![],
        env_vars: std::collections::HashMap::new(),
        executable_path: None,
        report_level: None,
        auto_fix: false,
        check: true,
    }
}

#[test]
fn test_djlint_can_handle() {
    let linter = DjLint::new();

    // Should handle HTML files
    assert!(linter.can_handle(Path::new("test.html")));
    assert!(linter.can_handle(Path::new("test.djhtml")));
    assert!(linter.can_handle(Path::new("test.jinja")));
    assert!(linter.can_handle(Path::new("test.j2")));
    assert!(linter.can_handle(Path::new("test.hbs")));

    // Should not handle other files
    assert!(!linter.can_handle(Path::new("test.txt")));
    assert!(!linter.can_handle(Path::new("test.py")));
    assert!(!linter.can_handle(Path::new("test")));
}

#[test]
fn test_djlint_parse_output() {
    let linter = DjLint::new();

    let output = r#"
core/templates/admin/form.html
───────────────────────────────────────────────────────────────────────────────
H021 14:8 Inline styles should be avoided. <div style=
T003 46:0 Endblock should have name. {% endblock %}

core/templates/base.html
───────────────────────────────────────────────────────────────────────────────
H020 213:6 Empty tag pair found. Consider removing. <div></div>
"#;

    let issues = linter.parse_output(output, "");

    assert_eq!(issues.len(), 3);

    // Check first issue
    assert_eq!(issues[0].severity, IssueSeverity::Warning);
    assert_eq!(issues[0].code, Some("H021".to_string()));
    assert_eq!(issues[0].line, Some(14));
    assert_eq!(issues[0].column, Some(8));
    assert_eq!(
        issues[0].file.as_ref().unwrap().to_str().unwrap(),
        "core/templates/admin/form.html"
    );

    // Check second issue
    assert_eq!(issues[1].severity, IssueSeverity::Error);
    assert_eq!(issues[1].code, Some("T003".to_string()));
    assert_eq!(issues[1].line, Some(46));
    assert_eq!(issues[1].column, Some(0));

    // Check third issue
    assert_eq!(issues[2].severity, IssueSeverity::Warning);
    assert_eq!(issues[2].code, Some("H020".to_string()));
    assert_eq!(
        issues[2].file.as_ref().unwrap().to_str().unwrap(),
        "core/templates/base.html"
    );
}

#[test]
fn test_djlint_formatter_can_handle() {
    let formatter = DjLintFormatter::new();

    // Should handle HTML files
    assert!(formatter.can_handle(Path::new("test.html")));
    assert!(formatter.can_handle(Path::new("test.djhtml")));
    assert!(formatter.can_handle(Path::new("test.jinja")));
    assert!(formatter.can_handle(Path::new("test.j2")));
    assert!(formatter.can_handle(Path::new("test.hbs")));

    // Should not handle other files
    assert!(!formatter.can_handle(Path::new("test.txt")));
    assert!(!formatter.can_handle(Path::new("test.py")));
    assert!(!formatter.can_handle(Path::new("test")));
}

#[test]
fn test_djlint_integration() {
    // Skip if djlint is not installed
    if !utils::is_command_available("djlint") {
        println!("Skipping djlint integration test - djlint not installed");
        return;
    }

    let temp_dir = TempDir::new().unwrap();

    // Create a test file with formatting issues
    let test_content = r#"
<div>
    <div style="color: red">
        {% if user.is_authenticated %}
            <p>Welcome!</p>
        {% endif %}
    </div>
    <div></div>
</div>
"#;
    let file_path = create_test_file(&temp_dir, "test.html", test_content);

    // Test linting
    let linter = DjLint::new();
    let config = create_test_config();
    let result = linter.execute(&[file_path.clone()], &config).unwrap();

    // Should find issues (H021 for inline style, H020 for empty div)
    assert!(!result.success);
    assert!(!result.issues.is_empty());

    // Test formatting
    let formatter = DjLintFormatter::new();
    let mut format_config = create_test_config();
    format_config.auto_fix = true;

    let format_result = formatter
        .execute(&[file_path.clone()], &format_config)
        .unwrap();
    assert!(format_result.success);

    // Verify the file was formatted
    let formatted_content = fs::read_to_string(&file_path).unwrap();
    assert_ne!(formatted_content, test_content);
}

#[test]
fn test_djlint_with_django_template() {
    // Skip if djlint is not installed
    if !utils::is_command_available("djlint") {
        println!("Skipping djlint django template test - djlint not installed");
        return;
    }

    let temp_dir = TempDir::new().unwrap();

    // Create a test file with django template issues
    let test_content = r#"
{% extends "base.html" %}
{% block content %}
<div class="container">
    {% for item in items %}
    <div class="item">
        {{ item.name }}
    </div>
    {% endfor %}
</div>
{% endblock %}
"#;
    let file_path = create_test_file(&temp_dir, "test.djhtml", test_content);

    let linter = DjLint::new();
    let mut config = create_test_config();
    config.extra_args = vec!["--profile".to_string(), "django".to_string()];

    let result = linter.execute(&[file_path], &config).unwrap();

    // Should find T003 (unnamed endblock) issue
    assert!(!result.success);
    assert!(result
        .issues
        .iter()
        .any(|i| i.code == Some("T003".to_string())));
}

#[test]
fn test_empty_file_list() {
    let linter = DjLint::new();
    let formatter = DjLintFormatter::new();
    let config = create_test_config();

    // Both tools should succeed with empty file list
    let lint_result = linter.execute(&[], &config).unwrap();
    assert!(lint_result.success);
    assert!(lint_result.issues.is_empty());

    let format_result = formatter.execute(&[], &config).unwrap();
    assert!(format_result.success);
    assert!(format_result.issues.is_empty());
}

#[test]
fn test_disabled_tools() {
    let linter = DjLint::new();
    let formatter = DjLintFormatter::new();
    let mut config = create_test_config();
    config.enabled = false;

    // Both tools should succeed when disabled
    let lint_result = linter
        .execute(&[PathBuf::from("test.html")], &config)
        .unwrap();
    assert!(lint_result.success);
    assert!(lint_result.issues.is_empty());

    let format_result = formatter
        .execute(&[PathBuf::from("test.html")], &config)
        .unwrap();
    assert!(format_result.success);
    assert!(format_result.issues.is_empty());
}
