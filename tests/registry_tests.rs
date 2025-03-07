use siren::models::{Language, ToolType};
use siren::tools::{DefaultToolRegistry, ToolRegistry};

mod test_mocks {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;

    use siren::errors::ToolError;
    use siren::models::{Language, LintResult, ToolConfig, ToolInfo, ToolType};
    use siren::tools::LintTool;

    /// A mock implementation of the LintTool trait for testing
    pub struct MockTool {
        name: String,
        languages: Vec<Language>,
        tool_type: ToolType,
        description: String,
        available: bool,
        version: Option<String>,
    }

    impl MockTool {
        /// Create a new mock tool
        pub fn new(
            name: &str,
            languages: Vec<Language>,
            tool_type: ToolType,
            description: &str,
            available: bool,
            version: Option<String>,
        ) -> Arc<Self> {
            Arc::new(Self {
                name: name.to_string(),
                languages,
                tool_type,
                description: description.to_string(),
                available,
                version,
            })
        }
    }

    impl LintTool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn can_handle(&self, file_path: &Path) -> bool {
            let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

            self.languages.iter().any(|lang| match lang {
                Language::Rust => extension == "rs",
                Language::Python => extension == "py",
                Language::JavaScript => extension == "js" || extension == "jsx",
                Language::TypeScript => extension == "ts" || extension == "tsx",
                _ => false,
            })
        }

        fn execute(
            &self,
            _files: &[PathBuf],
            _config: &ToolConfig,
        ) -> Result<LintResult, ToolError> {
            // Return a mock result
            Ok(LintResult {
                tool_name: self.name.clone(),
                tool: Some(ToolInfo {
                    name: self.name.clone(),
                    tool_type: self.tool_type,
                    languages: self.languages.clone(),
                    available: self.is_available(),
                    version: self.version(),
                    description: self.description().to_string(),
                }),
                success: true,
                issues: vec![],
                execution_time: Duration::from_millis(100),
                stdout: None,
                stderr: None,
            })
        }

        fn tool_type(&self) -> ToolType {
            self.tool_type
        }

        fn languages(&self) -> Vec<Language> {
            self.languages.clone()
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn is_available(&self) -> bool {
            self.available
        }

        fn version(&self) -> Option<String> {
            self.version.clone()
        }
    }
}

// Use our local MockTool implementation
use test_mocks::MockTool;

#[test]
fn test_registry_empty_on_creation() {
    let registry = DefaultToolRegistry::new();
    assert_eq!(registry.get_all_tools().len(), 0);
}

#[test]
fn test_registry_can_register_tool() {
    let mut registry = DefaultToolRegistry::new();

    // Create a mock tool
    let tool = MockTool::new(
        "mock_rustfmt",
        vec![Language::Rust],
        ToolType::Formatter,
        "A mock Rust formatter",
        true,
        Some("1.0.0".to_string()),
    );

    // Register the tool
    registry.register_tool(tool.clone());

    // Check that the tool was registered
    assert_eq!(registry.get_all_tools().len(), 1);

    // Check that we can retrieve the tool by name
    let retrieved_tool = registry.get_tool_by_name("mock_rustfmt");
    assert!(retrieved_tool.is_some());

    // Check that the retrieved tool has the right properties
    let retrieved_tool = retrieved_tool.unwrap();
    assert_eq!(retrieved_tool.name(), "mock_rustfmt");
    assert_eq!(retrieved_tool.languages().len(), 1);
    assert_eq!(retrieved_tool.tool_type(), ToolType::Formatter);
}

#[test]
fn test_registry_get_tools_by_language() {
    let mut registry = DefaultToolRegistry::new();

    // Create and register tools for different languages
    let rust_tool = MockTool::new(
        "mock_rustfmt",
        vec![Language::Rust],
        ToolType::Formatter,
        "A mock Rust formatter",
        true,
        Some("1.0.0".to_string()),
    );

    let python_tool = MockTool::new(
        "mock_black",
        vec![Language::Python],
        ToolType::Formatter,
        "A mock Python formatter",
        true,
        Some("22.1.0".to_string()),
    );

    registry.register_tool(rust_tool);
    registry.register_tool(python_tool);

    // Check that we can filter by language
    let rust_tools = registry.get_tools_for_language(Language::Rust);
    assert_eq!(rust_tools.len(), 1);
    assert_eq!(rust_tools[0].name(), "mock_rustfmt");

    let python_tools = registry.get_tools_for_language(Language::Python);
    assert_eq!(python_tools.len(), 1);
    assert_eq!(python_tools[0].name(), "mock_black");

    // Check that filtering for an unregistered language returns empty
    let js_tools = registry.get_tools_for_language(Language::JavaScript);
    assert_eq!(js_tools.len(), 0);
}

#[test]
fn test_registry_get_tools_by_type() {
    let mut registry = DefaultToolRegistry::new();

    // Create and register tools of different types
    let formatter = MockTool::new(
        "mock_rustfmt",
        vec![Language::Rust],
        ToolType::Formatter,
        "A mock Rust formatter",
        true,
        Some("1.0.0".to_string()),
    );

    let linter = MockTool::new(
        "mock_clippy",
        vec![Language::Rust],
        ToolType::Linter,
        "A mock Rust linter",
        true,
        Some("0.9.0".to_string()),
    );

    registry.register_tool(formatter);
    registry.register_tool(linter);

    // Check that we can filter by tool type
    let formatters = registry.get_tools_by_type(ToolType::Formatter);
    assert_eq!(formatters.len(), 1);
    assert_eq!(formatters[0].name(), "mock_rustfmt");

    let linters = registry.get_tools_by_type(ToolType::Linter);
    assert_eq!(linters.len(), 1);
    assert_eq!(linters[0].name(), "mock_clippy");

    // Check that filtering for an unregistered type returns empty
    let fixers = registry.get_tools_by_type(ToolType::Fixer);
    assert_eq!(fixers.len(), 0);
}

#[test]
fn test_registry_get_tools_for_language_and_type() {
    let mut registry = DefaultToolRegistry::new();

    // Create and register tools for different languages and types
    let rust_formatter = MockTool::new(
        "mock_rustfmt",
        vec![Language::Rust],
        ToolType::Formatter,
        "A mock Rust formatter",
        true,
        Some("1.0.0".to_string()),
    );

    let rust_linter = MockTool::new(
        "mock_clippy",
        vec![Language::Rust],
        ToolType::Linter,
        "A mock Rust linter",
        true,
        Some("0.9.0".to_string()),
    );

    let python_formatter = MockTool::new(
        "mock_black",
        vec![Language::Python],
        ToolType::Formatter,
        "A mock Python formatter",
        true,
        Some("22.1.0".to_string()),
    );

    let python_linter = MockTool::new(
        "mock_mypy",
        vec![Language::Python],
        ToolType::Linter,
        "A mock Python linter",
        true,
        Some("1.0.0".to_string()),
    );

    registry.register_tool(rust_formatter);
    registry.register_tool(rust_linter);
    registry.register_tool(python_formatter);
    registry.register_tool(python_linter);

    // Check that we can filter by language and type
    let rust_formatters =
        registry.get_tools_for_language_and_type(Language::Rust, ToolType::Formatter);
    assert_eq!(rust_formatters.len(), 1);
    assert_eq!(rust_formatters[0].name(), "mock_rustfmt");

    let rust_linters = registry.get_tools_for_language_and_type(Language::Rust, ToolType::Linter);
    assert_eq!(rust_linters.len(), 1);
    assert_eq!(rust_linters[0].name(), "mock_clippy");

    let python_formatters =
        registry.get_tools_for_language_and_type(Language::Python, ToolType::Formatter);
    assert_eq!(python_formatters.len(), 1);
    assert_eq!(python_formatters[0].name(), "mock_black");

    // Check that we can find Python linters
    let python_linters =
        registry.get_tools_for_language_and_type(Language::Python, ToolType::Linter);
    assert_eq!(python_linters.len(), 1);
    assert_eq!(python_linters[0].name(), "mock_mypy");
}
