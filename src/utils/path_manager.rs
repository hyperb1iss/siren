use std::collections::{HashMap, HashSet};
use std::io;
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
#[derive(Debug)]
pub struct PathManager {
    /// All collected files
    files: Vec<PathBuf>,
    /// Files grouped by extension
    files_by_extension: HashMap<String, Vec<PathBuf>>,
    /// Files grouped by language
    files_by_language: HashMap<Language, Vec<PathBuf>>,
    /// Path contexts (e.g., project roots)
    contexts: Vec<PathContext>,
    /// Flag to indicate if paths are discovered or explicitly provided
    is_discovered: bool,
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

        // If no paths provided, use current directory
        let paths_to_process = if paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            paths.to_vec()
        };

        // For explicit paths, only add the specific paths provided
        if !self.is_discovered {
            // Process each path but don't expand directories
            for path in &paths_to_process {
                if path.is_file() {
                    // If it's a file, add it directly
                    self.add_file(path.clone());
                } else if path.is_dir() {
                    // If it's a directory, add just the directory itself without expansion
                    self.add_file(path.clone());
                }
            }
            return Ok(self);
        }

        // For discovered paths, process each path recursively
        for path in &paths_to_process {
            if path.is_file() {
                // If it's a file, add it directly
                self.add_file(path.clone());
            } else if path.is_dir() {
                // If it's a directory, collect all files recursively
                match self.collect_files_from_directory(path) {
                    Ok(files) => {
                        self.add_files(files);
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
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

    /// Organize files into contexts (e.g., project roots)
    pub fn organize_contexts(&mut self) -> &mut Self {
        // Clear existing contexts
        self.contexts.clear();

        // Find Rust projects (Cargo.toml)
        self.organize_rust_contexts();

        // Find Python projects (pyproject.toml, setup.py)
        self.organize_python_contexts();

        // Find JS projects (package.json)
        self.organize_js_contexts();

        // Add remaining files to a default context
        self.organize_remaining_files();

        self
    }

    /// Organize Rust files into contexts based on Cargo.toml
    fn organize_rust_contexts(&mut self) {
        let rust_files: Vec<PathBuf> = self
            .files_by_language
            .get(&Language::Rust)
            .cloned()
            .unwrap_or_default();

        if rust_files.is_empty() {
            return;
        }

        // Track which files have been assigned to a context
        let mut assigned_files = HashSet::new();

        // Find Cargo.toml for each Rust file
        for file in &rust_files {
            if assigned_files.contains(file) {
                continue;
            }

            if let Some(cargo_dir) = self.find_cargo_toml_dir(file) {
                // Find all Rust files that belong to this Cargo.toml
                let context_files: Vec<PathBuf> = rust_files
                    .iter()
                    .filter(|f| {
                        !assigned_files.contains(*f) && self.is_in_cargo_project(f, &cargo_dir)
                    })
                    .cloned()
                    .collect();

                // Mark these files as assigned
                for f in &context_files {
                    assigned_files.insert(f.clone());
                }

                // Create a new context
                let context = PathContext {
                    root: cargo_dir,
                    files: context_files,
                    language: Some(Language::Rust),
                    metadata: HashMap::new(),
                };

                self.contexts.push(context);
            }
        }
    }

    /// Find the directory containing Cargo.toml for a Rust file
    fn find_cargo_toml_dir(&self, file_path: &Path) -> Option<PathBuf> {
        let mut current_dir = file_path.parent()?;

        loop {
            let cargo_path = current_dir.join("Cargo.toml");
            if cargo_path.exists() {
                return Some(current_dir.to_path_buf());
            }

            // Move up one directory
            if let Some(parent) = current_dir.parent() {
                current_dir = parent;
            } else {
                break;
            }
        }

        None
    }

    /// Check if a file is in a Cargo project
    fn is_in_cargo_project(&self, file_path: &Path, cargo_dir: &Path) -> bool {
        if let Some(parent) = file_path.parent() {
            parent.starts_with(cargo_dir)
        } else {
            false
        }
    }

    /// Organize Python files into contexts
    fn organize_python_contexts(&mut self) {
        let python_files: Vec<PathBuf> = self
            .files_by_language
            .get(&Language::Python)
            .cloned()
            .unwrap_or_default();

        if python_files.is_empty() {
            return;
        }

        // Track which files have been assigned to a context
        let mut assigned_files = HashSet::new();

        // Find Python project roots
        for file in &python_files {
            if assigned_files.contains(file) {
                continue;
            }

            if let Some(project_dir) = self.find_python_project_dir(file) {
                // Find all Python files that belong to this project
                let context_files: Vec<PathBuf> = python_files
                    .iter()
                    .filter(|f| {
                        !assigned_files.contains(*f) && self.is_in_python_project(f, &project_dir)
                    })
                    .cloned()
                    .collect();

                // Mark these files as assigned
                for f in &context_files {
                    assigned_files.insert(f.clone());
                }

                // Create a new context
                let context = PathContext {
                    root: project_dir,
                    files: context_files,
                    language: Some(Language::Python),
                    metadata: HashMap::new(),
                };

                self.contexts.push(context);
            }
        }
    }

    /// Find Python project directory
    fn find_python_project_dir(&self, file_path: &Path) -> Option<PathBuf> {
        let mut current_dir = file_path.parent()?;

        loop {
            // Check for pyproject.toml
            if current_dir.join("pyproject.toml").exists() {
                return Some(current_dir.to_path_buf());
            }

            // Check for setup.py
            if current_dir.join("setup.py").exists() {
                return Some(current_dir.to_path_buf());
            }

            // Move up one directory
            if let Some(parent) = current_dir.parent() {
                current_dir = parent;
            } else {
                break;
            }
        }

        None
    }

    /// Check if a file is in a Python project
    fn is_in_python_project(&self, file_path: &Path, project_dir: &Path) -> bool {
        if let Some(parent) = file_path.parent() {
            parent.starts_with(project_dir)
        } else {
            false
        }
    }

    /// Organize JavaScript files into contexts
    fn organize_js_contexts(&mut self) {
        let js_files: Vec<PathBuf> = self
            .files_by_language
            .get(&Language::JavaScript)
            .cloned()
            .unwrap_or_default();

        if js_files.is_empty() {
            return;
        }

        // Track which files have been assigned to a context
        let mut assigned_files = HashSet::new();

        // Find JS project roots
        for file in &js_files {
            if assigned_files.contains(file) {
                continue;
            }

            if let Some(project_dir) = self.find_js_project_dir(file) {
                // Find all JS files that belong to this project
                let context_files: Vec<PathBuf> = js_files
                    .iter()
                    .filter(|f| {
                        !assigned_files.contains(*f) && self.is_in_js_project(f, &project_dir)
                    })
                    .cloned()
                    .collect();

                // Mark these files as assigned
                for f in &context_files {
                    assigned_files.insert(f.clone());
                }

                // Create a new context
                let context = PathContext {
                    root: project_dir,
                    files: context_files,
                    language: Some(Language::JavaScript),
                    metadata: HashMap::new(),
                };

                self.contexts.push(context);
            }
        }
    }

    /// Find JavaScript project directory
    fn find_js_project_dir(&self, file_path: &Path) -> Option<PathBuf> {
        let mut current_dir = file_path.parent()?;

        loop {
            // Check for package.json
            if current_dir.join("package.json").exists() {
                return Some(current_dir.to_path_buf());
            }

            // Move up one directory
            if let Some(parent) = current_dir.parent() {
                current_dir = parent;
            } else {
                break;
            }
        }

        None
    }

    /// Check if a file is in a JavaScript project
    fn is_in_js_project(&self, file_path: &Path, project_dir: &Path) -> bool {
        if let Some(parent) = file_path.parent() {
            parent.starts_with(project_dir)
        } else {
            false
        }
    }

    /// Organize remaining files into a default context
    fn organize_remaining_files(&mut self) {
        // Get all files that haven't been assigned to a context
        let assigned_files: HashSet<_> = self
            .contexts
            .iter()
            .flat_map(|ctx| ctx.files.iter().cloned())
            .collect();

        let remaining_files: Vec<_> = self
            .files
            .iter()
            .filter(|f| !assigned_files.contains(*f))
            .cloned()
            .collect();

        if !remaining_files.is_empty() {
            // Find a common root for remaining files
            if let Some(common_root) = self.find_common_root(&remaining_files) {
                let context = PathContext {
                    root: common_root,
                    files: remaining_files,
                    language: None,
                    metadata: HashMap::new(),
                };
                self.contexts.push(context);
            }
        }
    }

    /// Find common root directory for a set of files
    fn find_common_root(&self, files: &[PathBuf]) -> Option<PathBuf> {
        if files.is_empty() {
            return None;
        }

        // Start with the parent of the first file
        let first_parent = files[0].parent()?.to_path_buf();

        // Find the common prefix of all file paths
        let mut common_prefix = first_parent;

        for file in files.iter().skip(1) {
            if let Some(parent) = file.parent() {
                // Find common prefix between current common_prefix and this file's parent
                common_prefix = self.find_common_prefix(&common_prefix, parent);

                // If we've reduced to root, stop
                if common_prefix.as_os_str().is_empty() {
                    return Some(PathBuf::from("/"));
                }
            }
        }

        Some(common_prefix)
    }

    /// Find common prefix between two paths
    fn find_common_prefix(&self, path1: &Path, path2: &Path) -> PathBuf {
        let components1: Vec<_> = path1.components().collect();
        let components2: Vec<_> = path2.components().collect();

        let mut common = PathBuf::new();

        for (c1, c2) in components1.iter().zip(components2.iter()) {
            if c1 == c2 {
                common.push(c1.as_os_str());
            } else {
                break;
            }
        }

        common
    }

    /// Get files for a specific tool
    pub fn get_files_for_tool<T: LintTool + ?Sized>(&self, tool: &T) -> Vec<PathBuf> {
        // If these are explicitly provided paths (not discovered), respect them completely
        if !self.is_discovered {
            return self.files.clone();
        }

        // For discovered files, apply language and tool filtering
        let tool_languages = tool.languages();
        let language_files: HashSet<_> = tool_languages
            .iter()
            .flat_map(|lang| {
                self.files_by_language
                    .get(lang)
                    .cloned()
                    .unwrap_or_default()
            })
            .collect();

        // Then filter by tool's can_handle method
        language_files
            .into_iter()
            .filter(|file| tool.can_handle(file))
            .collect()
    }

    /// Get optimized paths for a tool
    pub fn get_optimized_paths_for_tool<T: LintTool + ?Sized>(&self, tool: &T) -> Vec<PathBuf> {
        // If paths are explicitly provided, just return them directly without optimization
        if !self.is_discovered {
            return self.files.clone();
        }

        // Only do path optimization for discovered files
        let files = self.get_files_for_tool(tool);

        // Apply our own optimization
        self.optimize_paths(&files)
    }

    /// Optimize paths by collapsing directories when possible
    fn optimize_paths(&self, files: &[PathBuf]) -> Vec<PathBuf> {
        if files.is_empty() {
            return Vec::new();
        }

        // Group files by directory
        let mut dir_files: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

        for file in files {
            if let Some(parent) = file.parent() {
                dir_files
                    .entry(parent.to_path_buf())
                    .or_default()
                    .push(file.clone());
            } else {
                // If no parent, keep the file as is
                dir_files
                    .entry(PathBuf::from("."))
                    .or_default()
                    .push(file.clone());
            }
        }

        // Optimize by collapsing directories when all files in a directory are included
        let mut optimized = Vec::new();

        for (dir, dir_files) in dir_files {
            // Count total files in this directory
            let total_files_in_dir = match self.count_files_in_dir(&dir) {
                Ok(count) => count,
                Err(_) => {
                    // If we can't count, just add all files individually
                    optimized.extend(dir_files);
                    continue;
                }
            };

            // If all files in the directory are included, just add the directory
            if dir_files.len() == total_files_in_dir && total_files_in_dir > 1 {
                optimized.push(dir);
            } else {
                // Otherwise, add individual files
                optimized.extend(dir_files);
            }
        }

        optimized
    }

    /// Count the number of files in a directory
    fn count_files_in_dir(&self, dir: &Path) -> Result<usize, io::Error> {
        let mut count = 0;

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                count += 1;
            }
        }

        Ok(count)
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

    /// Check if this path manager is for discovered paths (vs explicit paths)
    pub fn is_discovered(&self) -> bool {
        self.is_discovered
    }

    /// Create a new PathManager with explicit paths
    pub fn with_explicit_paths(paths: Vec<PathBuf>) -> Self {
        Self {
            files: paths,
            files_by_extension: HashMap::new(),
            files_by_language: HashMap::new(),
            contexts: Vec::new(),
            is_discovered: false,
        }
    }

    /// Create a new PathManager for discovered paths
    pub fn for_discovered_paths() -> Self {
        Self {
            files: Vec::new(),
            files_by_extension: HashMap::new(),
            files_by_language: HashMap::new(),
            contexts: Vec::new(),
            is_discovered: true,
        }
    }
}

impl Default for PathManager {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            files_by_extension: HashMap::new(),
            files_by_language: HashMap::new(),
            contexts: Vec::new(),
            is_discovered: true, // Default to discovered mode
        }
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
