use crate::models::Language;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Detect language from file contents
pub fn detect_language_from_content(file_path: &Path) -> Option<Language> {
    if let Ok(content) = fs::read_to_string(file_path) {
        let content = content.to_lowercase();

        // Check file content for language-specific patterns
        if content.contains("<?php") {
            return Some(Language::Php);
        }

        if content.contains("<!doctype html") || content.contains("<html") {
            return Some(Language::Html);
        }

        // For Python scripts with shebang
        if content.starts_with("#!/usr/bin/env python") || content.starts_with("#!/usr/bin/python")
        {
            return Some(Language::Python);
        }

        // For JavaScript scripts with shebang
        if content.starts_with("#!/usr/bin/env node") {
            return Some(Language::JavaScript);
        }

        // For Ruby scripts with shebang
        if content.starts_with("#!/usr/bin/env ruby") || content.starts_with("#!/usr/bin/ruby") {
            return Some(Language::Ruby);
        }
    }

    None
}

/// Find most likely language for a file without extension
pub fn detect_language_for_extensionless_file(file_path: &Path) -> Option<Language> {
    // First try content-based detection
    if let Some(lang) = detect_language_from_content(file_path) {
        return Some(lang);
    }

    // Check common file names
    if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
        match file_name {
            "Dockerfile" => Some(Language::Docker),
            "Makefile" => Some(Language::Makefile),
            "Gemfile" | "Rakefile" => Some(Language::Ruby),
            "package.json" | "tsconfig.json" => Some(Language::JavaScript),
            "requirements.txt" | "setup.py" => Some(Language::Python),
            "Cargo.toml" => Some(Language::Rust),
            "go.mod" | "go.sum" => Some(Language::Go),
            _ => None,
        }
    } else {
        None
    }
}

/// Try to detect the language from the filename pattern
pub fn detect_language_from_filename(file_path: &Path) -> Option<Language> {
    // First check extensions using the Language enum method
    if let Some(lang) = Language::from_path(file_path) {
        return Some(lang);
    }

    // For files without extension, use heuristics
    if file_path.extension().is_none() {
        return detect_language_for_extensionless_file(file_path);
    }

    None
}

/// Count lines of code in a file
pub fn count_lines_of_code(file_path: &Path) -> usize {
    match fs::read_to_string(file_path) {
        Ok(content) => content.lines().count(),
        Err(_) => 0,
    }
}

/// Find files of a specific language
pub fn find_files_of_language(dir: &Path, language: Language, max_depth: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();

    let walker = WalkDir::new(dir)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok());

    for entry in walker {
        let path = entry.path();

        // Skip directories
        if !path.is_file() {
            continue;
        }

        // Check if this file is of the target language
        if Language::from_path(path) == Some(language) {
            files.push(path.to_path_buf());
        }
    }

    files
}

/// Find repository root (containing .git, etc.)
pub fn find_repository_root(dir: &Path) -> Option<PathBuf> {
    let mut current_dir = dir.to_path_buf();

    loop {
        // Check for .git directory
        let git_dir = current_dir.join(".git");
        if git_dir.exists() && git_dir.is_dir() {
            return Some(current_dir);
        }

        // Try parent directory, break if no parent
        if !current_dir.pop() {
            break;
        }
    }

    None
}
