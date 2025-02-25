//! Project detection functionality

use crate::errors::{DetectionError, SirenError};
use crate::models::{DetectedTool, Framework, Language, ProjectInfo, ToolType};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod framework_detection;
mod language_detection;
mod tool_detection;

/// Trait for detecting project information
pub trait ProjectDetector {
    /// Detect project information from a directory
    fn detect(&self, dir: &Path) -> Result<ProjectInfo, SirenError>;
}

/// Default implementation of ProjectDetector
pub struct DefaultProjectDetector {
    /// Maximum depth to scan
    max_depth: usize,
}

impl DefaultProjectDetector {
    /// Create a new DefaultProjectDetector
    pub fn new() -> Self {
        Self { max_depth: 5 }
    }

    /// Create a new DefaultProjectDetector with a custom max depth
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self { max_depth }
    }

    /// Detect language based on file extension
    fn detect_language_from_extension(&self, ext: &str) -> Option<Language> {
        match ext.to_lowercase().as_str() {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "js" => Some(Language::JavaScript),
            "ts" => Some(Language::TypeScript),
            "jsx" => Some(Language::JavaScript),
            "tsx" => Some(Language::TypeScript),
            "html" | "htm" => Some(Language::Html),
            "css" => Some(Language::Css),
            "go" => Some(Language::Go),
            "rb" => Some(Language::Ruby),
            "java" => Some(Language::Java),
            "php" => Some(Language::Php),
            "c" => Some(Language::C),
            "cpp" | "cc" | "cxx" => Some(Language::Cpp),
            "h" | "hpp" => Some(Language::Cpp),
            "cs" => Some(Language::CSharp),
            "swift" => Some(Language::Swift),
            "md" | "markdown" => Some(Language::Markdown),
            "json" => Some(Language::Json),
            "yml" | "yaml" => Some(Language::Yaml),
            "toml" => Some(Language::Toml),
            _ => None,
        }
    }

    /// Detect framework based on files in the project
    fn detect_frameworks(&self, dir: &Path) -> Vec<Framework> {
        let mut frameworks = Vec::new();

        // Check for React
        if dir.join("package.json").exists() {
            if let Ok(content) = std::fs::read_to_string(dir.join("package.json")) {
                if content.contains("\"react\"") {
                    frameworks.push(Framework::React);
                }
                if content.contains("\"vue\"") {
                    frameworks.push(Framework::Vue);
                }
                if content.contains("\"angular\"") {
                    frameworks.push(Framework::Angular);
                }
            }
        }

        // Check for Django
        if dir.join("manage.py").exists() && dir.join("settings.py").exists() {
            frameworks.push(Framework::Django);
        }

        // Check for Flask
        if dir.join("app.py").exists() || dir.join("wsgi.py").exists() {
            if let Ok(content) = std::fs::read_to_string(dir.join("app.py")) {
                if content.contains("from flask import") {
                    frameworks.push(Framework::Flask);
                }
            }
        }

        // Check for Rails
        if dir.join("Gemfile").exists() {
            if let Ok(content) = std::fs::read_to_string(dir.join("Gemfile")) {
                if content.contains("gem 'rails'") || content.contains("gem \"rails\"") {
                    frameworks.push(Framework::Rails);
                }
            }
        }

        frameworks
    }

    /// Detect tools based on configuration files
    fn detect_tools(&self, dir: &Path) -> Vec<DetectedTool> {
        let mut tools = Vec::new();

        // Check for rustfmt
        if let Some(path) = self.find_file(dir, "rustfmt.toml") {
            tools.push(DetectedTool {
                name: "rustfmt".to_string(),
                config_path: path,
                tool_type: crate::models::ToolType::Formatter,
                language: Language::Rust,
            });
        }

        // Check for clippy
        if let Some(path) = self.find_file(dir, "clippy.toml") {
            tools.push(DetectedTool {
                name: "clippy".to_string(),
                config_path: path,
                tool_type: crate::models::ToolType::Linter,
                language: Language::Rust,
            });
        }

        // Check for black
        if let Some(path) = self.find_file(dir, "pyproject.toml") {
            // TODO: Check if the file actually contains black configuration
            tools.push(DetectedTool {
                name: "black".to_string(),
                config_path: path,
                tool_type: crate::models::ToolType::Formatter,
                language: Language::Python,
            });
        }

        // Check for eslint
        let eslint_path = self
            .find_file(dir, ".eslintrc")
            .or_else(|| self.find_file(dir, ".eslintrc.json"))
            .or_else(|| self.find_file(dir, ".eslintrc.js"));

        if let Some(path) = eslint_path {
            tools.push(DetectedTool {
                name: "eslint".to_string(),
                config_path: path,
                tool_type: ToolType::Linter,
                language: Language::JavaScript,
            });
        }

        // Check for prettier
        let prettier_path = self
            .find_file(dir, ".prettierrc")
            .or_else(|| self.find_file(dir, ".prettierrc.json"))
            .or_else(|| self.find_file(dir, ".prettierrc.js"));

        if let Some(path) = prettier_path {
            tools.push(DetectedTool {
                name: "prettier".to_string(),
                config_path: path,
                tool_type: ToolType::Formatter,
                language: Language::JavaScript,
            });
        }

        tools
    }

    /// Find a file by name in the directory or its parents
    fn find_file(&self, dir: &Path, filename: &str) -> Option<PathBuf> {
        let mut current_dir = Some(dir);

        while let Some(dir) = current_dir {
            let file_path = dir.join(filename);
            if file_path.exists() {
                return Some(file_path);
            }
            current_dir = dir.parent();
        }

        None
    }
}

impl ProjectDetector for DefaultProjectDetector {
    fn detect(&self, dir: &Path) -> Result<ProjectInfo, SirenError> {
        if !dir.exists() || !dir.is_dir() {
            return Err(DetectionError::InvalidDirectory(dir.to_path_buf()).into());
        }

        let mut languages = HashMap::new();
        let mut file_count = 0;

        // Walk directory tree and count files by language
        let walker = walkdir::WalkDir::new(dir)
            .max_depth(self.max_depth)
            .into_iter()
            .filter_entry(|e| {
                // Skip hidden directories
                let filename = e.file_name().to_string_lossy();
                !filename.starts_with(".") || filename == "."
            });

        for entry in walker.filter_map(Result::ok) {
            if entry.file_type().is_file() {
                file_count += 1;

                if let Some(ext) = entry.path().extension() {
                    if let Some(lang) =
                        self.detect_language_from_extension(ext.to_string_lossy().as_ref())
                    {
                        *languages.entry(lang).or_insert(0) += 1;
                    }
                }
            }
        }

        // If no files found, return error
        if file_count == 0 {
            return Err(
                DetectionError::DetectionFailed("No files found in directory".to_string()).into(),
            );
        }

        // Detect frameworks
        let frameworks = self.detect_frameworks(dir);

        // Detect tools
        let detected_tools = self.detect_tools(dir);

        // Create language list sorted by file count (most common first)
        let mut language_list: Vec<_> = languages.keys().cloned().collect();
        language_list.sort_by(|a, b| {
            let count_a = languages.get(a).unwrap_or(&0);
            let count_b = languages.get(b).unwrap_or(&0);
            count_b.cmp(count_a)
        });

        Ok(ProjectInfo {
            languages: language_list,
            frameworks,
            file_counts: languages,
            detected_tools,
        })
    }
}
