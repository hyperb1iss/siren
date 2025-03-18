//! Project detection functionality

use crate::errors::{DetectionError, SirenError};
use crate::models::{DetectedTool, Framework, Language, ProjectInfo};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod tool_detection;

/// Trait for detecting project information
pub trait ProjectDetector {
    /// Detect project information from paths
    ///
    /// Returns a tuple containing:
    /// - ProjectInfo: Information about the detected project
    /// - Vec<PathBuf>: List of files collected during detection (respecting .gitignore)
    fn detect(&self, paths: &[PathBuf]) -> Result<(ProjectInfo, Vec<PathBuf>), SirenError>;

    /// Detect project information with specific path patterns
    ///
    /// Returns a tuple containing:
    /// - ProjectInfo: Information about the detected project
    /// - Vec<PathBuf>: List of files collected during detection (respecting .gitignore)
    fn detect_with_patterns(
        &self,
        dir: &Path,
        patterns: &[String],
    ) -> Result<(ProjectInfo, Vec<PathBuf>), SirenError>;
}

/// Default implementation of ProjectDetector
#[derive(Clone)]
pub struct DefaultProjectDetector {}

impl Default for DefaultProjectDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultProjectDetector {
    /// Create a new DefaultProjectDetector
    pub fn new() -> Self {
        Self {}
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
    fn detect(&self, paths: &[PathBuf]) -> Result<(ProjectInfo, Vec<PathBuf>), SirenError> {
        // Create a collection to track the languages we detect
        let mut languages = HashMap::new();
        let mut detected_tools = Vec::new();
        let mut file_count = 0;
        let mut collected_files = Vec::new();

        // If no paths are provided, use the current directory
        let paths_to_process = if paths.is_empty() {
            log::debug!("No paths provided for detection, using current directory");
            vec![PathBuf::from(".")]
        } else {
            paths.to_vec()
        };

        // Process each path (directory, specific file, or pattern)
        for path in &paths_to_process {
            if path.is_file() {
                // For a single file, just detect its language
                file_count += 1;
                collected_files.push(path.clone());

                if let Some(ext) = path.extension() {
                    if let Some(lang) =
                        self.detect_language_from_extension(ext.to_string_lossy().as_ref())
                    {
                        *languages.entry(lang).or_insert(0) += 1;
                    }
                }
            } else if path.is_dir() {
                // For directories, use the existing collect_files_with_gitignore function
                let dir_files = crate::utils::collect_files_with_gitignore(path)?;

                for file_path in &dir_files {
                    file_count += 1;
                    collected_files.push(file_path.clone());

                    if let Some(ext) = file_path.extension() {
                        if let Some(lang) =
                            self.detect_language_from_extension(ext.to_string_lossy().as_ref())
                        {
                            *languages.entry(lang).or_insert(0) += 1;
                        }
                    }
                }

                // Detect tools for this directory
                let dir_tools = self.detect_tools(path);
                for tool in dir_tools {
                    if !detected_tools.contains(&tool) {
                        detected_tools.push(tool);
                    }
                }
            } else {
                return Err(DetectionError::InvalidDirectory(path.to_path_buf()).into());
            }
        }

        // If no files found, return error
        if file_count == 0 {
            return Err(DetectionError::DetectionFailed(
                "No files found in specified paths".to_string(),
            )
            .into());
        }

        // Detect frameworks - only if we're looking at directories
        let frameworks = if paths_to_process.iter().any(|p| p.is_dir()) {
            // Use the first directory for framework detection
            let default_path = PathBuf::from(".");
            let first_dir = paths_to_process
                .iter()
                .find(|p| p.is_dir())
                .unwrap_or(&default_path);
            self.detect_frameworks(first_dir)
        } else {
            Vec::new()
        };

        // Create language list sorted by file count (most common first)
        let mut language_list: Vec<_> = languages.keys().cloned().collect();
        language_list.sort_by(|a, b| {
            let count_a = languages.get(a).unwrap_or(&0);
            let count_b = languages.get(b).unwrap_or(&0);
            count_b.cmp(count_a)
        });

        let project_info = ProjectInfo {
            languages: language_list,
            frameworks,
            file_counts: languages,
            detected_tools,
        };

        Ok((project_info, collected_files))
    }

    fn detect_with_patterns(
        &self,
        dir: &Path,
        patterns: &[String],
    ) -> Result<(ProjectInfo, Vec<PathBuf>), SirenError> {
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
        let mut collected_files = Vec::new();

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
            collected_files.push(file_path.clone());

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
            // Use the existing collect_files_with_gitignore function
            let dir_files = crate::utils::collect_files_with_gitignore(&base_dir)?;

            for file_path in dir_files {
                // Skip if we're looking at specific files and this isn't one of them
                if !specific_files.is_empty() && !specific_files.contains(&file_path) {
                    continue;
                }

                file_count += 1;
                collected_files.push(file_path.clone());

                if let Some(ext) = file_path.extension() {
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

        let project_info = ProjectInfo {
            languages: language_list,
            frameworks,
            file_counts: languages,
            detected_tools,
        };

        Ok((project_info, collected_files))
    }
}
