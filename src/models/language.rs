use std::path::Path;

use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString};

/// Languages supported by Siren
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
    Sequence,
)]
#[strum(serialize_all = "lowercase")]
pub enum Language {
    /// Rust programming language
    Rust,

    /// Python programming language
    Python,

    /// JavaScript programming language
    JavaScript,

    /// TypeScript programming language
    TypeScript,

    /// HTML markup language
    Html,

    /// CSS styling language
    Css,

    /// Go programming language
    Go,

    /// Ruby programming language
    Ruby,

    /// PHP programming language
    Php,

    /// Dockerfile
    Docker,

    /// Makefile
    Makefile,

    /// Java programming language
    Java,

    /// C programming language
    C,

    /// C++ programming language
    Cpp,

    /// C# programming language
    CSharp,

    /// Swift programming language
    Swift,

    /// Markdown markup language
    Markdown,

    /// JSON data format
    Json,

    /// YAML data format
    Yaml,

    /// TOML data format
    Toml,
}

impl Language {
    /// Get the file extensions associated with this language
    pub fn extensions(&self) -> &[&str] {
        match self {
            Language::Rust => &["rs"],
            Language::Python => &["py", "pyi", "pyx"],
            Language::JavaScript => &["js", "jsx", "mjs", "cjs"],
            Language::TypeScript => &["ts", "tsx"],
            Language::Html => &["html", "htm", "xhtml", "djhtml"],
            Language::Css => &["css", "scss", "sass", "less"],
            Language::Go => &["go"],
            Language::Ruby => &["rb", "rake", "gemspec"],
            Language::Php => &["php", "phtml", "php3", "php4", "php5", "php7", "phps"],
            Language::Docker => &[],   // No extension, detected by filename
            Language::Makefile => &[], // No extension, detected by filename
            Language::Java => &["java"],
            Language::C => &["c", "h"],
            Language::Cpp => &["cpp", "cc", "cxx", "hpp", "hxx", "h"],
            Language::CSharp => &["cs"],
            Language::Swift => &["swift"],
            Language::Markdown => &["md", "markdown"],
            Language::Json => &["json"],
            Language::Yaml => &["yml", "yaml"],
            Language::Toml => &["toml"],
        }
    }

    /// Try to detect language from file path
    pub fn from_path(path: &Path) -> Option<Self> {
        // Check for files without extensions first
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            match filename {
                "Dockerfile" => return Some(Language::Docker),
                "Makefile" => return Some(Language::Makefile),
                _ => {}
            }
        }

        // Check by extension
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            let extension = extension.to_lowercase();

            // Check each language's extensions
            for lang in enum_iterator::all::<Language>() {
                if lang.extensions().contains(&extension.as_str()) {
                    return Some(lang);
                }
            }
        }

        None
    }

    /// Get the emoji representation of this language
    pub fn emoji(&self) -> &'static str {
        match self {
            Language::Rust => "🦀",
            Language::Python => "🐍",
            Language::JavaScript => "🌐",
            Language::TypeScript => "📘",
            Language::Html => "🌐",
            Language::Css => "🎨",
            Language::Go => "🐹",
            Language::Ruby => "💎",
            Language::Php => "🐘",
            Language::Docker => "🐳",
            Language::Makefile => "🏗️",
            Language::Java => "☕",
            Language::C => "🔍",
            Language::Cpp => "🔧",
            Language::CSharp => "🔷",
            Language::Swift => "🔶",
            Language::Markdown => "📝",
            Language::Json => "📋",
            Language::Yaml => "📄",
            Language::Toml => "📁",
        }
    }
}

/// Frameworks supported by Siren
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum Framework {
    /// React JavaScript framework
    React,

    /// Vue JavaScript framework
    Vue,

    /// Angular JavaScript framework
    Angular,

    /// Django Python framework
    Django,

    /// Flask Python framework
    Flask,

    /// Rails Ruby framework
    Rails,
}

impl Framework {
    /// Get the emoji representation of this framework
    pub fn emoji(&self) -> &'static str {
        match self {
            Framework::React => "⚛️",
            Framework::Vue => "🟢",
            Framework::Angular => "🔴",
            Framework::Django => "🎸",
            Framework::Flask => "🍶",
            Framework::Rails => "🚂",
        }
    }

    /// Get the language associated with this framework
    pub fn language(&self) -> Language {
        match self {
            Framework::React | Framework::Vue | Framework::Angular => Language::JavaScript,
            Framework::Django | Framework::Flask => Language::Python,
            Framework::Rails => Language::Ruby,
        }
    }
}
