use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::errors::SirenError;
use crate::models::Language;
use crate::tools::LintTool;
use crate::utils;

/// PathContext stores information about a group of paths
/// that share the same context (e.g., same project root)
#[derive(Debug)]
pub struct PathContext {
    /// Root directory for this context
    pub root: PathBuf,
    /// Files in this context
    pub files: Vec<PathBuf>,
    /// Language detected for this context
    pub language: Option<Language>,
    /// Additional context-specific metadata
    pub metadata: HashMap<String, String>,
}

/// PathManager handles the collection, organization, and optimization
/// of paths for tools to process
#[derive(Debug, Default)]
pub struct PathManager {
    /// All collected files
    files: Vec<PathBuf>,
    /// Files grouped by extension
    files_by_extension: HashMap<String, Vec<PathBuf>>,
    /// Files grouped by language
    files_by_language: HashMap<Language, Vec<PathBuf>>,
    /// Path contexts (e.g., project roots)
    contexts: Vec<PathContext>,
}

impl PathManager {
    /// Create a new PathManager
    pub fn new() -> Self {
        Self::default()
    }

    /// Collect files from provided paths, respecting git_modified flag
    pub fn collect_files(
        &mut self,
        paths: &[PathBuf],
        git_modified_only: bool,
    ) -> Result<&mut Self, SirenError> {
        // If git_modified_only is true, get only git modified files
        if git_modified_only {
            if let Ok(git_files) = utils::get_git_modified_files(&PathBuf::from(".")) {
                self.add_files(git_files);
                return Ok(self);
            } else {
                return Err(SirenError::Detection(
                    crate::errors::DetectionError::DetectionFailed(
                        "Failed to get git modified files".to_string(),
                    ),
                ));
            }
        }

        // Case 2: If explicit paths were provided via args, we use them directly
        if !paths.is_empty() {
            log::debug!("Using explicitly provided paths: {:?}", paths);
            // When paths are provided explicitly, we always treat them as-is without any special handling
            for path in paths {
                self.add_file(path.clone());
            }
            return Ok(self);
        }

        // Case 1: No paths provided - scan current directory
        log::debug!("No paths provided, scanning current directory");
        // Get the absolute path for current directory to avoid path resolution issues
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        log::debug!(
            "Using absolute current directory: {}",
            current_dir.display()
        );

        // First get all files from subdirectories to find which ones contain relevant files
        let mut subdirectories = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&current_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                    // Skip hidden directories and common non-source directories
                    if dir_name.starts_with('.')
                        || dir_name == "node_modules"
                        || dir_name == "venv"
                        || dir_name == "target"
                    {
                        continue;
                    }

                    // Check if this directory contains any supported files
                    if let Ok(files) = self.collect_files_from_directory(&path) {
                        if !files.is_empty() {
                            // Check if any files have recognized extensions
                            let has_recognized_files = files.iter().any(|f| {
                                if let Some(ext) = f.extension().and_then(|e| e.to_str()) {
                                    self.detect_language_from_extension(ext).is_some()
                                } else {
                                    false
                                }
                            });

                            if has_recognized_files {
                                subdirectories.push(path);
                            }
                        }
                    }
                }
            }
        }

        if !subdirectories.is_empty() {
            log::debug!(
                "Found {} subdirectories containing relevant files:",
                subdirectories.len()
            );
            for dir in &subdirectories {
                log::debug!("  - {}", dir.display());
            }

            // Use these subdirectories as our paths
            self.files.clear();
            self.files_by_extension.clear();
            self.files_by_language.clear();

            for dir in subdirectories {
                self.add_file(dir);
            }
        } else {
            // If no relevant subdirectories found, just use the current directory
            log::debug!("No relevant subdirectories found, using current directory");
            self.files.clear();
            self.files_by_extension.clear();
            self.files_by_language.clear();
            self.add_file(current_dir);
        }

        Ok(self)
    }

    /// Collect files from a directory recursively
    fn collect_files_from_directory(&self, dir: &Path) -> Result<Vec<PathBuf>, SirenError> {
        let mut files = Vec::new();

        // Check if .gitignore exists and use it to filter files
        let walker = ignore::WalkBuilder::new(dir)
            .hidden(false) // Don't skip hidden files
            .git_global(false) // Don't use global gitignore
            .git_ignore(true) // Use .gitignore if present
            .build();

        for result in walker {
            match result {
                Ok(entry) => {
                    let path = entry.path().to_path_buf();
                    if path.is_file() {
                        files.push(path);
                    }
                }
                Err(e) => {
                    return Err(SirenError::Detection(
                        crate::errors::DetectionError::DetectionFailed(format!(
                            "File system error: {}",
                            e
                        )),
                    ));
                }
            }
        }

        Ok(files)
    }

    /// Add files to the manager
    pub fn add_files(&mut self, files: Vec<PathBuf>) -> &mut Self {
        for file in files {
            self.add_file(file);
        }
        self
    }

    /// Add a single file to the manager
    pub fn add_file(&mut self, file: PathBuf) -> &mut Self {
        // Skip if already added
        if self.files.contains(&file) {
            return self;
        }

        // Add to main files list
        self.files.push(file.clone());

        // Add to extension map
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            self.files_by_extension
                .entry(ext.to_string())
                .or_default()
                .push(file.clone());
        }

        // Add to language map based on extension
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            if let Some(language) = self.detect_language_from_extension(ext) {
                self.files_by_language
                    .entry(language)
                    .or_default()
                    .push(file);
            }
        }

        self
    }

    /// Detect language from file extension
    fn detect_language_from_extension(&self, ext: &str) -> Option<Language> {
        match ext {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" => Some(Language::JavaScript),
            "html" | "htm" | "djhtml" | "jinja" | "jinja2" => Some(Language::Html),
            "css" | "scss" | "sass" | "less" => Some(Language::Css),
            "json" => Some(Language::Json),
            "md" | "markdown" => Some(Language::Markdown),
            "yaml" | "yml" => Some(Language::Yaml),
            "toml" => Some(Language::Toml),
            _ => None,
        }
    }

    /// Get optimized paths for a tool
    pub fn get_optimized_paths_for_tool<T: LintTool + ?Sized>(&self, _tool: &T) -> Vec<PathBuf> {
        // Just return the files directly - no need to optimize since we're
        // keeping directories as-is when provided as explicit arguments
        self.files.clone()
    }

    /// Get all collected files
    pub fn get_all_files(&self) -> &[PathBuf] {
        &self.files
    }

    /// Get files by language
    pub fn get_files_by_language(&self, language: Language) -> Vec<PathBuf> {
        self.files_by_language
            .get(&language)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all contexts
    pub fn get_all_contexts(&self) -> &[PathContext] {
        &self.contexts
    }
}

impl Clone for PathContext {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            files: self.files.clone(),
            language: self.language,
            metadata: self.metadata.clone(),
        }
    }
}
