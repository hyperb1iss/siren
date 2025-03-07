use crate::config::{GeneralConfig, LanguageConfig, SirenConfig, StyleConfig};
use crate::models::{Language, ToolConfig, ToolType};
use std::collections::HashMap;

/// Create a default configuration for a new project
pub fn create_default_config() -> SirenConfig {
    let mut config = SirenConfig::default();

    // Add default language configs
    config
        .languages
        .insert(Language::Rust, create_rust_config());
    config
        .languages
        .insert(Language::Python, create_python_config());
    config
        .languages
        .insert(Language::JavaScript, create_javascript_config());
    config
        .languages
        .insert(Language::TypeScript, create_typescript_config());

    // Add default tool configs
    add_default_rust_tools(&mut config.tools);
    add_default_python_tools(&mut config.tools);
    add_default_javascript_tools(&mut config.tools);

    config
}

/// Create default Rust language config
fn create_rust_config() -> LanguageConfig {
    LanguageConfig {
        enabled: true,
        line_length: 100,
        ignore_rules: vec![],
        additional_extensions: vec![],
    }
}

/// Create default Python language config
fn create_python_config() -> LanguageConfig {
    LanguageConfig {
        enabled: true,
        line_length: 88,                                            // Black default
        ignore_rules: vec!["E203".to_string(), "W503".to_string()], // Common ruff/flake8 ignores
        additional_extensions: vec![],
    }
}

/// Create default JavaScript language config
fn create_javascript_config() -> LanguageConfig {
    LanguageConfig {
        enabled: true,
        line_length: 80,
        ignore_rules: vec![],
        additional_extensions: vec![],
    }
}

/// Create default TypeScript language config
fn create_typescript_config() -> LanguageConfig {
    LanguageConfig {
        enabled: true,
        line_length: 80,
        ignore_rules: vec![],
        additional_extensions: vec![],
    }
}

/// Add default Rust tools
fn add_default_rust_tools(tools: &mut HashMap<String, ToolConfig>) {
    // rustfmt
    tools.insert(
        "rustfmt".to_string(),
        ToolConfig {
            enabled: true,
            extra_args: vec![],
            env_vars: HashMap::new(),
            executable_path: None,
            report_level: Some("error".to_string()),
            auto_fix: false,
        },
    );

    // clippy
    tools.insert(
        "clippy".to_string(),
        ToolConfig {
            enabled: true,
            extra_args: vec!["--".to_string(), "-D".to_string(), "warnings".to_string()],
            env_vars: HashMap::new(),
            executable_path: None,
            report_level: Some("error".to_string()),
            auto_fix: false,
        },
    );

    // cargo fix
    tools.insert(
        "cargo-fix".to_string(),
        ToolConfig {
            enabled: true,
            extra_args: vec![],
            env_vars: HashMap::new(),
            executable_path: None,
            report_level: Some("error".to_string()),
            auto_fix: true,
        },
    );
}

/// Add default Python tools
fn add_default_python_tools(tools: &mut HashMap<String, ToolConfig>) {
    // black
    tools.insert(
        "black".to_string(),
        ToolConfig {
            enabled: true,
            extra_args: vec![],
            env_vars: HashMap::new(),
            executable_path: None,
            report_level: Some("error".to_string()),
            auto_fix: false,
        },
    );

    // ruff
    tools.insert(
        "ruff".to_string(),
        ToolConfig {
            enabled: true,
            extra_args: vec![],
            env_vars: HashMap::new(),
            executable_path: None,
            report_level: Some("error".to_string()),
            auto_fix: false,
        },
    );

    // mypy
    tools.insert(
        "mypy".to_string(),
        ToolConfig {
            enabled: true,
            extra_args: vec!["--ignore-missing-imports".to_string()],
            env_vars: HashMap::new(),
            executable_path: None,
            report_level: Some("error".to_string()),
            auto_fix: false,
        },
    );
}

/// Add default JavaScript tools
fn add_default_javascript_tools(tools: &mut HashMap<String, ToolConfig>) {
    // prettier
    tools.insert(
        "prettier".to_string(),
        ToolConfig {
            enabled: true,
            extra_args: vec![],
            env_vars: HashMap::new(),
            executable_path: None,
            report_level: Some("error".to_string()),
            auto_fix: false,
        },
    );

    // eslint
    tools.insert(
        "eslint".to_string(),
        ToolConfig {
            enabled: true,
            extra_args: vec![],
            env_vars: HashMap::new(),
            executable_path: None,
            report_level: Some("error".to_string()),
            auto_fix: false,
        },
    );
}

/// Create a strict config for CI environments
pub fn create_ci_config() -> GeneralConfig {
    GeneralConfig {
        fail_level: "warning".to_string(), // Fail on warnings in CI
        use_relative_paths: true,
        default_paths: vec![".".to_string()],
        auto_fix: false,          // Don't auto-fix in CI
        git_modified_only: false, // Check everything in CI
    }
}
