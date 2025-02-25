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

    /// Unknown language
    Unknown,
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
