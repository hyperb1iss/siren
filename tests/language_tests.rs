use rstest::rstest;
use std::path::Path;

// We need to make sure we're importing from the crate being tested
use siren::models::Language;

#[test]
fn test_language_from_extension() {
    // Test cases for various file extensions
    assert_eq!(
        Language::from_path(Path::new("test.rs")),
        Some(Language::Rust)
    );
    assert_eq!(
        Language::from_path(Path::new("test.py")),
        Some(Language::Python)
    );
    assert_eq!(
        Language::from_path(Path::new("test.js")),
        Some(Language::JavaScript)
    );
    assert_eq!(
        Language::from_path(Path::new("test.ts")),
        Some(Language::TypeScript)
    );
    assert_eq!(
        Language::from_path(Path::new("test.html")),
        Some(Language::Html)
    );
    assert_eq!(
        Language::from_path(Path::new("test.css")),
        Some(Language::Css)
    );
    assert_eq!(
        Language::from_path(Path::new("test.go")),
        Some(Language::Go)
    );
    assert_eq!(
        Language::from_path(Path::new("test.rb")),
        Some(Language::Ruby)
    );
    assert_eq!(
        Language::from_path(Path::new("test.php")),
        Some(Language::Php)
    );
    assert_eq!(
        Language::from_path(Path::new("test.java")),
        Some(Language::Java)
    );
}

#[test]
fn test_language_from_filename() {
    // Test special filenames without extensions
    assert_eq!(
        Language::from_path(Path::new("Dockerfile")),
        Some(Language::Docker)
    );
    assert_eq!(
        Language::from_path(Path::new("Makefile")),
        Some(Language::Makefile)
    );

    // Test with path components
    assert_eq!(
        Language::from_path(Path::new("/path/to/Dockerfile")),
        Some(Language::Docker)
    );
    assert_eq!(
        Language::from_path(Path::new("src/Makefile")),
        Some(Language::Makefile)
    );
}

#[test]
fn test_language_from_unknown_extension() {
    // Test with unknown extensions
    assert_eq!(Language::from_path(Path::new("test.unknown")), None);
    assert_eq!(Language::from_path(Path::new("test")), None);
    assert_eq!(Language::from_path(Path::new("")), None);
}

#[rstest]
#[case(Language::Rust, &["rs"])]
#[case(Language::Python, &["py", "pyi", "pyx"])]
#[case(Language::JavaScript, &["js", "jsx", "mjs", "cjs"])]
#[case(Language::TypeScript, &["ts", "tsx"])]
fn test_language_extensions(#[case] language: Language, #[case] expected_extensions: &[&str]) {
    assert_eq!(language.extensions(), expected_extensions);
}

#[rstest]
#[case(Language::Rust, "🦀")]
#[case(Language::Python, "🐍")]
#[case(Language::JavaScript, "🌐")]
#[case(Language::TypeScript, "📘")]
fn test_language_emoji(#[case] language: Language, #[case] expected_emoji: &str) {
    assert_eq!(language.emoji(), expected_emoji);
}
