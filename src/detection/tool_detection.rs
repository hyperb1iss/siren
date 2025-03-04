use crate::models::{DetectedTool, Language, ToolType};
use globset::{Glob, GlobSet, GlobSetBuilder};
use log::debug;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Detect tool configurations in a project directory
pub fn detect_tools(dir: &Path) -> Vec<DetectedTool> {
    detect_tools_in_paths(dir, &[])
}

/// Detect tool configurations in specific paths with glob pattern support
///
/// # Arguments
///
/// * `project_root` - The root directory of the project
/// * `patterns` - Glob patterns to include (e.g., "core/templates/**/*.html")
///
/// If patterns is empty, all files/directories under project_root will be checked.
/// Patterns are relative to the project_root.
pub fn detect_tools_in_paths(project_root: &Path, patterns: &[String]) -> Vec<DetectedTool> {
    let mut tools = Vec::new();

    // If no patterns provided, just check the project root
    if patterns.is_empty() {
        // Check for Rust tools
        detect_rust_tools(project_root, &mut tools);

        // Check for Python tools
        detect_python_tools(project_root, &mut tools);

        // Check for JavaScript/TypeScript tools
        detect_js_tools(project_root, &mut tools);

        // Check for CSS tools
        detect_css_tools(project_root, &mut tools);

        // Check for HTML tools
        detect_html_tools(project_root, &mut tools);

        return tools;
    }

    // Create a globset from the provided patterns
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        match Glob::new(pattern) {
            Ok(glob) => {
                builder.add(glob);
            }
            Err(err) => {
                debug!("Invalid glob pattern '{}': {}", pattern, err);
            }
        }
    }

    let globset = match builder.build() {
        Ok(gs) => gs,
        Err(err) => {
            debug!("Failed to build globset: {}", err);
            return tools;
        }
    };

    // Collect paths that match the patterns
    let matching_paths = collect_matching_paths(project_root, &globset);

    // Group matching paths by directory to avoid redundant checks
    let directories: HashSet<PathBuf> = matching_paths
        .iter()
        .filter_map(|path| {
            if path.is_dir() {
                Some(path.clone())
            } else {
                path.parent().map(|p| p.to_path_buf())
            }
        })
        .collect();

    // Check for tools in each matching directory
    for dir in directories {
        // Check for Rust tools
        detect_rust_tools(&dir, &mut tools);

        // Check for Python tools
        detect_python_tools(&dir, &mut tools);

        // Check for JavaScript/TypeScript tools
        detect_js_tools(&dir, &mut tools);

        // Check for CSS tools
        detect_css_tools(&dir, &mut tools);

        // Check for HTML tools
        detect_html_tools(&dir, &mut tools);
    }

    // Deduplicate tools (we might have found the same tool config multiple times)
    deduplicate_tools(tools)
}

/// Collect paths that match the given glob patterns
fn collect_matching_paths(root: &Path, globset: &GlobSet) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok);

    for entry in walker {
        // Get the path relative to the root
        if let Some(relative_path) = pathdiff::diff_paths(entry.path(), root) {
            // Convert to string for matching
            if let Some(path_str) = relative_path.to_str() {
                // Check if path matches any of the glob patterns
                if globset.is_match(path_str) {
                    result.push(entry.path().to_path_buf());
                }
            }
        }
    }

    result
}

/// Deduplicate tools by name and config_path
fn deduplicate_tools(tools: Vec<DetectedTool>) -> Vec<DetectedTool> {
    let mut unique_tools = Vec::new();
    let mut seen = HashSet::new();

    for tool in tools {
        let key = (
            tool.name.clone(),
            tool.config_path.to_string_lossy().to_string(),
        );
        if !seen.contains(&key) {
            seen.insert(key);
            unique_tools.push(tool);
        }
    }

    unique_tools
}

/// Detect Rust linting/formatting tools
fn detect_rust_tools(dir: &Path, tools: &mut Vec<DetectedTool>) {
    // Check for rustfmt.toml
    let rustfmt_config = dir.join("rustfmt.toml");
    if rustfmt_config.exists() {
        debug!("Detected rustfmt configuration");
        tools.push(DetectedTool {
            name: "rustfmt".to_string(),
            config_path: rustfmt_config,
            tool_type: ToolType::Formatter,
            language: Language::Rust,
        });
    }

    // Check for .rustfmt.toml
    let alt_rustfmt_config = dir.join(".rustfmt.toml");
    if alt_rustfmt_config.exists() {
        debug!("Detected rustfmt configuration (.rustfmt.toml)");
        tools.push(DetectedTool {
            name: "rustfmt".to_string(),
            config_path: alt_rustfmt_config,
            tool_type: ToolType::Formatter,
            language: Language::Rust,
        });
    }

    // Check for clippy.toml
    let clippy_config = dir.join("clippy.toml");
    if clippy_config.exists() {
        debug!("Detected clippy configuration");
        tools.push(DetectedTool {
            name: "clippy".to_string(),
            config_path: clippy_config,
            tool_type: ToolType::Linter,
            language: Language::Rust,
        });
    }

    // Check for .clippy.toml
    let alt_clippy_config = dir.join(".clippy.toml");
    if alt_clippy_config.exists() {
        debug!("Detected clippy configuration (.clippy.toml)");
        tools.push(DetectedTool {
            name: "clippy".to_string(),
            config_path: alt_clippy_config,
            tool_type: ToolType::Linter,
            language: Language::Rust,
        });
    }
}

/// Detect Python linting/formatting tools
fn detect_python_tools(dir: &Path, tools: &mut Vec<DetectedTool>) {
    // Check for Black configuration in pyproject.toml
    let pyproject_toml = dir.join("pyproject.toml");
    if pyproject_toml.exists() {
        // We'll assume black is configured if pyproject.toml exists
        // In a real implementation, we would parse the file to confirm
        debug!("Detected potential Black configuration in pyproject.toml");
        tools.push(DetectedTool {
            name: "black".to_string(),
            config_path: pyproject_toml.clone(),
            tool_type: ToolType::Formatter,
            language: Language::Python,
        });

        // Check for Ruff in pyproject.toml
        debug!("Detected potential Ruff configuration in pyproject.toml");
        tools.push(DetectedTool {
            name: "ruff".to_string(),
            config_path: pyproject_toml.clone(),
            tool_type: ToolType::Linter,
            language: Language::Python,
        });
    }

    // Check for .pylintrc
    let pylintrc = dir.join(".pylintrc");
    if pylintrc.exists() {
        debug!("Detected pylint configuration");
        tools.push(DetectedTool {
            name: "pylint".to_string(),
            config_path: pylintrc,
            tool_type: ToolType::Linter,
            language: Language::Python,
        });
    }

    // Check for pylintrc (no dot)
    let alt_pylintrc = dir.join("pylintrc");
    if alt_pylintrc.exists() {
        debug!("Detected pylint configuration (pylintrc)");
        tools.push(DetectedTool {
            name: "pylint".to_string(),
            config_path: alt_pylintrc,
            tool_type: ToolType::Linter,
            language: Language::Python,
        });
    }

    // Check for mypy.ini
    let mypy_ini = dir.join("mypy.ini");
    if mypy_ini.exists() {
        debug!("Detected mypy configuration");
        tools.push(DetectedTool {
            name: "mypy".to_string(),
            config_path: mypy_ini,
            tool_type: ToolType::Linter,
            language: Language::Python,
        });
    }

    // Check for .mypy.ini
    let alt_mypy_ini = dir.join(".mypy.ini");
    if alt_mypy_ini.exists() {
        debug!("Detected mypy configuration (.mypy.ini)");
        tools.push(DetectedTool {
            name: "mypy".to_string(),
            config_path: alt_mypy_ini,
            tool_type: ToolType::Linter,
            language: Language::Python,
        });
    }

    // Check for ruff.toml
    let ruff_toml = dir.join("ruff.toml");
    if ruff_toml.exists() {
        debug!("Detected ruff configuration (ruff.toml)");
        tools.push(DetectedTool {
            name: "ruff".to_string(),
            config_path: ruff_toml,
            tool_type: ToolType::Linter,
            language: Language::Python,
        });
    }
}

/// Detect JavaScript/TypeScript linting/formatting tools
fn detect_js_tools(dir: &Path, tools: &mut Vec<DetectedTool>) {
    // Check for .prettierrc (various formats)
    for ext in &[".json", ".yaml", ".yml", ".js", ".toml", ""] {
        let prettier_config = dir.join(format!(".prettierrc{}", ext));
        if prettier_config.exists() {
            debug!("Detected prettier configuration");
            tools.push(DetectedTool {
                name: "prettier".to_string(),
                config_path: prettier_config,
                tool_type: ToolType::Formatter,
                language: Language::JavaScript, // Also applies to TypeScript
            });
            break; // Only need to detect one prettier config
        }
    }

    // Check for prettier.config.js
    let alt_prettier_config = dir.join("prettier.config.js");
    if alt_prettier_config.exists() {
        debug!("Detected prettier configuration (prettier.config.js)");
        tools.push(DetectedTool {
            name: "prettier".to_string(),
            config_path: alt_prettier_config,
            tool_type: ToolType::Formatter,
            language: Language::JavaScript, // Also applies to TypeScript
        });
    }

    // Check for .eslintrc (various formats)
    for ext in &[".json", ".yaml", ".yml", ".js", ""] {
        let eslint_config = dir.join(format!(".eslintrc{}", ext));
        if eslint_config.exists() {
            debug!("Detected ESLint configuration");
            tools.push(DetectedTool {
                name: "eslint".to_string(),
                config_path: eslint_config,
                tool_type: ToolType::Linter,
                language: Language::JavaScript, // Also applies to TypeScript
            });
            break; // Only need to detect one ESLint config
        }
    }

    // Check for eslint.config.js
    let alt_eslint_config = dir.join("eslint.config.js");
    if alt_eslint_config.exists() {
        debug!("Detected ESLint configuration (eslint.config.js)");
        tools.push(DetectedTool {
            name: "eslint".to_string(),
            config_path: alt_eslint_config,
            tool_type: ToolType::Linter,
            language: Language::JavaScript, // Also applies to TypeScript
        });
    }

    // Check for tsconfig.json (TypeScript)
    let tsconfig = dir.join("tsconfig.json");
    if tsconfig.exists() {
        debug!("Detected TypeScript configuration");
        tools.push(DetectedTool {
            name: "typescript".to_string(),
            config_path: tsconfig,
            tool_type: ToolType::TypeChecker,
            language: Language::TypeScript,
        });
    }

    // Check for dprint.json
    let dprint_config = dir.join("dprint.json");
    if dprint_config.exists() {
        debug!("Detected dprint configuration");
        tools.push(DetectedTool {
            name: "dprint".to_string(),
            config_path: dprint_config,
            tool_type: ToolType::Formatter,
            language: Language::JavaScript, // Also applies to TypeScript
        });
    }
}

/// Detect CSS linting/formatting tools
fn detect_css_tools(dir: &Path, tools: &mut Vec<DetectedTool>) {
    // Check for .stylelintrc (various formats)
    for ext in &[".json", ".yaml", ".yml", ".js", ""] {
        let stylelint_config = dir.join(format!(".stylelintrc{}", ext));
        if stylelint_config.exists() {
            debug!("Detected stylelint configuration");
            tools.push(DetectedTool {
                name: "stylelint".to_string(),
                config_path: stylelint_config,
                tool_type: ToolType::Linter,
                language: Language::Css,
            });
            break; // Only need to detect one stylelint config
        }
    }

    // Check for stylelint.config.js
    let alt_stylelint_config = dir.join("stylelint.config.js");
    if alt_stylelint_config.exists() {
        debug!("Detected stylelint configuration (stylelint.config.js)");
        tools.push(DetectedTool {
            name: "stylelint".to_string(),
            config_path: alt_stylelint_config,
            tool_type: ToolType::Linter,
            language: Language::Css,
        });
    }
}

/// Detect HTML linting/formatting tools
fn detect_html_tools(dir: &Path, tools: &mut Vec<DetectedTool>) {
    // Check for .htmlhintrc
    let htmlhint_config = dir.join(".htmlhintrc");
    if htmlhint_config.exists() {
        debug!("Detected htmlhint configuration");
        tools.push(DetectedTool {
            name: "htmlhint".to_string(),
            config_path: htmlhint_config,
            tool_type: ToolType::Linter,
            language: Language::Html,
        });
    }

    // Check for .djlintrc
    let djlint_config = dir.join(".djlintrc");
    if djlint_config.exists() {
        debug!("Detected djlint configuration");
        tools.push(DetectedTool {
            name: "djlint".to_string(),
            config_path: djlint_config,
            tool_type: ToolType::Formatter,
            language: Language::Html,
        });
    }
}
