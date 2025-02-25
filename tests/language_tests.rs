use std::path::Path;

// We need to make sure we're importing from the crate being tested
use siren::models::Language;
use siren::utils;

#[test]
fn test_language_from_extension() {
    // Test cases for various file extensions
    assert_eq!(utils::detect_language(Path::new("test.rs")), Language::Rust);
    assert_eq!(
        utils::detect_language(Path::new("test.py")),
        Language::Python
    );
    assert_eq!(
        utils::detect_language(Path::new("test.js")),
        Language::JavaScript
    );
    assert_eq!(
        utils::detect_language(Path::new("test.ts")),
        Language::TypeScript
    );
    assert_eq!(
        utils::detect_language(Path::new("test.html")),
        Language::Html
    );
    assert_eq!(utils::detect_language(Path::new("test.css")), Language::Css);
    assert_eq!(utils::detect_language(Path::new("test.go")), Language::Go);
    assert_eq!(utils::detect_language(Path::new("test.rb")), Language::Ruby);
    assert_eq!(utils::detect_language(Path::new("test.php")), Language::Php);
    assert_eq!(
        utils::detect_language(Path::new("test.java")),
        Language::Java
    );
}

#[test]
fn test_language_from_filename() {
    // Test special filenames without extensions
    assert_eq!(
        utils::detect_language(Path::new("Dockerfile")),
        Language::Docker
    );
    assert_eq!(
        utils::detect_language(Path::new("Makefile")),
        Language::Makefile
    );

    // Test with path components
    assert_eq!(
        utils::detect_language(Path::new("/path/to/Dockerfile")),
        Language::Docker
    );
    assert_eq!(
        utils::detect_language(Path::new("src/Makefile")),
        Language::Makefile
    );
}

#[test]
fn test_language_from_unknown_extension() {
    // Test with unknown extensions
    assert_eq!(
        utils::detect_language(Path::new("test.unknown")),
        Language::Unknown
    );
    assert_eq!(utils::detect_language(Path::new("test")), Language::Unknown);
    assert_eq!(utils::detect_language(Path::new("")), Language::Unknown);
}

// The following tests are commented out because the methods they test have been removed
// in the refactor. If you need this functionality, you'll need to implement these methods
// or update the tests to use the new architecture.

/*
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
*/
