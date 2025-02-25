use std::fs::File;
use std::io::Write;

use siren::config::{ConfigProvider, SirenConfig, TomlConfigProvider};
use siren::models::Language;
use tempfile::TempDir;

/// Creates a temporary TOML config file with the given content
fn create_temp_config(content: &str) -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join(".siren.toml");

    let mut file = File::create(&config_path).expect("Failed to create config file");
    file.write_all(content.as_bytes())
        .expect("Failed to write config content");

    temp_dir
}

#[test]
fn test_default_config() {
    // Verify default configuration has expected values
    let default_config = SirenConfig::default();

    // Check general config defaults
    assert_eq!(default_config.general.fail_level, "error");
    assert!(default_config.general.use_relative_paths);

    // Check style config defaults
    assert_eq!(default_config.style.theme, "default");
    assert!(default_config.style.use_emoji);

    // Check that languages and tools maps are empty
    assert!(default_config.languages.is_empty());
    assert!(default_config.tools.is_empty());

    // Check output config defaults
    assert!(default_config.output.show_line_numbers);
    assert!(default_config.output.show_file_paths);
    assert_eq!(default_config.output.max_issues_per_category, usize::MAX);
    assert!(default_config.output.show_code_snippets);
}

#[test]
fn test_load_toml_config() {
    // Create a temporary config file
    let config_content = r#"
    [general]
    fail_level = "warning"
    use_relative_paths = false
    
    [style]
    theme = "ocean"
    use_emoji = false
    
    [languages.Rust]
    line_length = 100
    ignore_rules = ["unused_variables", "dead_code"]
    
    [tools.rustfmt]
    enabled = true
    extra_args = ["--edition", "2021"]
    
    [output]
    show_line_numbers = false
    show_file_paths = true
    max_issues_per_category = 5
    show_code_snippets = true
    "#;

    let temp_dir = create_temp_config(config_content);

    // Load the config
    let provider = TomlConfigProvider::new();
    let config = provider
        .load_config(temp_dir.path())
        .expect("Failed to load config");

    // Verify the loaded config has the expected values
    assert_eq!(config.general.fail_level, "warning");
    assert!(!config.general.use_relative_paths);

    assert_eq!(config.style.theme, "ocean");
    assert!(!config.style.use_emoji);

    // Check language config
    let rust_config = config
        .languages
        .get(&Language::Rust)
        .expect("Rust config not found");
    assert_eq!(rust_config.line_length, Some(100));
    assert_eq!(
        rust_config.ignore_rules,
        Some(vec![
            "unused_variables".to_string(),
            "dead_code".to_string()
        ])
    );

    // Check tool config
    let rustfmt_config = config
        .tools
        .get("rustfmt")
        .expect("rustfmt config not found");
    assert!(rustfmt_config.enabled);
    assert_eq!(
        rustfmt_config.extra_args,
        Some(vec!["--edition".to_string(), "2021".to_string()])
    );

    // Check output config
    assert!(!config.output.show_line_numbers);
    assert_eq!(config.output.max_issues_per_category, 5);
    assert!(config.output.show_file_paths);
    assert!(config.output.show_code_snippets);
}

#[test]
fn test_config_not_found() {
    // Use a non-existent directory
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Load the config - should fall back to defaults
    let provider = TomlConfigProvider::new();
    let config = provider
        .load_config(temp_dir.path())
        .expect("Failed to load config");

    // Verify it loaded the default config
    assert_eq!(config.general.fail_level, "error"); // Default value
    assert_eq!(config.style.theme, "default"); // Default value
    assert!(config.languages.is_empty());
    assert!(config.tools.is_empty());
}

#[test]
fn test_partial_config() {
    // Create a config with only some fields specified
    let config_content = r#"
    [general]
    fail_level = "info"
    use_relative_paths = true
    
    [tools.pylint]
    enabled = false
    "#;

    let temp_dir = create_temp_config(config_content);

    // Load the config
    let provider = TomlConfigProvider::new();
    let config = provider
        .load_config(temp_dir.path())
        .expect("Failed to load config");

    // Verify the specified fields were loaded
    assert_eq!(config.general.fail_level, "info");

    let pylint_config = config.tools.get("pylint").expect("pylint config not found");
    assert!(!pylint_config.enabled);

    // Verify unspecified fields have default values
    assert!(config.general.use_relative_paths); // Default value
    assert_eq!(config.style.theme, "default"); // Default value
    assert!(config.style.use_emoji); // Default value
    assert!(config.languages.is_empty());
}

// TODO: Add tests for config merging once implemented
