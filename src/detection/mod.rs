//! Project detection functionality

use crate::errors::{DetectionError, SirenError};
use crate::models::{DetectedTool, Framework, Language, ProjectInfo};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod tool_detection;

/// Trait for detecting project information
pub trait ProjectDetector {
    /// Detect project information from a directory
    fn detect(&self, dir: &Path) -> Result<ProjectInfo, SirenError>;

    /// Detect project information with specific path patterns
    fn detect_with_patterns(
        &self,
        dir: &Path,
        patterns: &[String],
    ) -> Result<ProjectInfo, SirenError>;
}

/// Default implementation of ProjectDetector
#[derive(Clone)]
pub struct DefaultProjectDetector {
    /// Maximum depth to scan
    max_depth: usize,
}

impl Default for DefaultProjectDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultProjectDetector {
    /// Create a new DefaultProjectDetector
    pub fn new() -> Self {
        Self { max_depth: 5 }
    }

    /// Detect language based on file extension
    pub fn detect_language_from_extension(&self, ext: &str) -> Option<Language> {
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
        tool_detection::detect_tools(dir)
    }

    /// Detect tools based on configuration files in specific paths
    fn detect_tools_with_patterns(&self, dir: &Path, patterns: &[String]) -> Vec<DetectedTool> {
        tool_detection::detect_tools_in_paths(dir, patterns)
    }
}

impl ProjectDetector for DefaultProjectDetector {
    fn detect(&self, dir: &Path) -> Result<ProjectInfo, SirenError> {
        // Check if the path exists
        if !dir.exists() {
            return Err(DetectionError::InvalidDirectory(dir.to_path_buf()).into());
        }

        // If the "directory" is actually a file, use its parent directory
        // and call detect_with_patterns with the filename
        if dir.is_file() {
            // Get the absolute path to make sure we can reliably get the parent
            let absolute_path = if dir.is_absolute() {
                dir.to_path_buf()
            } else {
                std::env::current_dir()
                    .map_err(DetectionError::Io)?
                    .join(dir)
            };

            // Get the parent directory
            let parent_dir = absolute_path
                .parent()
                .ok_or_else(|| DetectionError::InvalidDirectory(dir.to_path_buf()))?;

            // Make sure the parent directory exists
            if !parent_dir.exists() || !parent_dir.is_dir() {
                return Err(DetectionError::InvalidDirectory(parent_dir.to_path_buf()).into());
            }

            // Get the filename as a string for the pattern
            let filename = absolute_path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .ok_or_else(|| DetectionError::InvalidDirectory(dir.to_path_buf()))?;

            // Use the detect_with_patterns method with the filename as a pattern
            println!(
                "Detecting single file: {} in directory: {}",
                filename,
                parent_dir.display()
            );
            return self.detect_with_patterns(parent_dir, &[filename]);
        }

        if !dir.is_dir() {
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

    fn detect_with_patterns(
        &self,
        dir: &Path,
        patterns: &[String],
    ) -> Result<ProjectInfo, SirenError> {
        // Check if the directory exists
        if !dir.exists() {
            return Err(DetectionError::InvalidDirectory(dir.to_path_buf()).into());
        }

        // If directory is actually a file, use its parent directory as the base
        let base_dir = if dir.is_file() {
            dir.parent()
                .map(|p| p.to_path_buf())
                .ok_or_else(|| DetectionError::InvalidDirectory(dir.to_path_buf()))?
        } else if !dir.is_dir() {
            return Err(DetectionError::InvalidDirectory(dir.to_path_buf()).into());
        } else {
            dir.to_path_buf()
        };

        let mut languages = HashMap::new();
        let mut file_count = 0;

        // Walk directory tree and count files by language
        let walker = walkdir::WalkDir::new(&base_dir)
            .max_depth(self.max_depth)
            .into_iter()
            .filter_entry(|e| {
                // Skip hidden directories
                let filename = e.file_name().to_string_lossy();
                !filename.starts_with(".") || filename == "."
            });

        // First check if any of the paths are specific files
        let mut specific_files = Vec::new();
        let mut has_glob_patterns = false;

        for pattern in patterns {
            // Check if this is a glob pattern
            if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
                has_glob_patterns = true;
                continue;
            }

            // Check if it's a specific file
            let path = if Path::new(pattern).is_absolute() {
                PathBuf::from(pattern)
            } else {
                base_dir.join(pattern)
            };

            if path.is_file() {
                specific_files.push(path);
            }
        }

        // If we have specific files, count them by language
        for file_path in &specific_files {
            file_count += 1;

            if let Some(ext) = file_path.extension() {
                if let Some(lang) =
                    self.detect_language_from_extension(ext.to_string_lossy().as_ref())
                {
                    *languages.entry(lang).or_insert(0) += 1;
                }
            }
        }

        // If we also have glob patterns or no specific files were found, scan the directory
        if has_glob_patterns || specific_files.is_empty() {
            for entry in walker.filter_map(Result::ok) {
                if entry.file_type().is_file() {
                    // Skip if we're looking at specific files and this isn't one of them
                    if !specific_files.is_empty()
                        && !specific_files.contains(&entry.path().to_path_buf())
                    {
                        continue;
                    }

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
        }

        // If no files found, return error
        if file_count == 0 {
            return Err(DetectionError::DetectionFailed(
                "No files found matching the specified patterns".to_string(),
            )
            .into());
        }

        // Detect frameworks - only if we're looking at a directory
        let frameworks = if specific_files.is_empty() {
            self.detect_frameworks(&base_dir)
        } else {
            Vec::new()
        };

        // Detect tools with patterns
        let detected_tools = self.detect_tools_with_patterns(&base_dir, patterns);

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
