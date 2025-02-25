use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use siren::models::{Language, ToolConfig, ToolType};
use siren::tools::{DefaultToolRegistry, ToolExecutor, ToolRegistry};

// Since we can't easily access another test module, use the mocks module directly
// Import our mock tool
use self::test_mocks::MockTool;

// Define a module with mock implementations for this test file
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
        language: Language,
        tool_type: ToolType,
        description: String,
        available: bool,
        version: Option<String>,
        priority: usize,
    }

    impl MockTool {
        /// Create a new mock tool
        pub fn new(
            name: &str,
            language: Language,
            tool_type: ToolType,
            description: &str,
            available: bool,
            version: Option<String>,
            priority: usize,
        ) -> Arc<Self> {
            Arc::new(Self {
                name: name.to_string(),
                language,
                tool_type,
                description: description.to_string(),
                available,
                version,
                priority,
            })
        }
    }

    impl LintTool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn can_handle(&self, file_path: &Path) -> bool {
            let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

            match self.language {
                Language::Rust => extension == "rs",
                Language::Python => extension == "py",
                Language::JavaScript => extension == "js" || extension == "jsx",
                Language::TypeScript => extension == "ts" || extension == "tsx",
                _ => false,
            }
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
                    language: self.language,
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

        fn language(&self) -> Language {
            self.language
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

        fn priority(&self) -> usize {
            self.priority
        }
    }
}

#[tokio::test]
async fn test_executor_run_tool() {
    // Create a registry and executor
    let mut registry = DefaultToolRegistry::new();

    // Create a mock tool
    let tool = MockTool::new(
        "mock_rustfmt",
        Language::Rust,
        ToolType::Formatter,
        "A mock Rust formatter",
        true,
        Some("1.0.0".to_string()),
        0,
    );

    // Create a dummy ToolConfig
    let config = ToolConfig {
        enabled: true,
        extra_args: Vec::new(),
        env_vars: HashMap::new(),
        executable_path: None,
        report_level: None,
        auto_fix: false,
        check: false,
    };

    // Create a dummy file path
    let files = vec![PathBuf::from("test.rs")];

    // Run the tool
    let executor = ToolExecutor::new(registry);
    let result = executor.run_tool(tool, &files, &config).await;

    // Check that execution succeeded
    assert!(result.is_ok());

    // Check the result details
    let result = result.unwrap();
    assert_eq!(result.tool_name, "mock_rustfmt");
    assert!(result.issues.is_empty());
}

#[tokio::test]
async fn test_executor_run_tools_for_language() {
    // Create a registry and executor
    let mut registry = DefaultToolRegistry::new();

    // Register tools for different languages
    let rust_tool = MockTool::new(
        "mock_rustfmt",
        Language::Rust,
        ToolType::Formatter,
        "A mock Rust formatter",
        true,
        Some("1.0.0".to_string()),
        0,
    );

    let python_tool = MockTool::new(
        "mock_black",
        Language::Python,
        ToolType::Formatter,
        "A mock Python formatter",
        true,
        Some("22.1.0".to_string()),
        0,
    );

    registry.register_tool(rust_tool);
    registry.register_tool(python_tool);

    // Create a dummy ToolConfig
    let config = ToolConfig {
        enabled: true,
        extra_args: Vec::new(),
        env_vars: HashMap::new(),
        executable_path: None,
        report_level: None,
        auto_fix: false,
        check: false,
    };

    // Create dummy file paths
    let rust_files = vec![PathBuf::from("test.rs")];

    // Create executor
    let executor = ToolExecutor::new(registry);

    // Run tools for Rust language
    let results = executor
        .run_tools_for_language(Language::Rust, &rust_files, &config)
        .await;

    // Check that we got one result for the one Rust tool
    assert_eq!(results.len(), 1);
    assert!(results[0].is_ok());

    // Check the result details
    let result = &results[0].as_ref().unwrap();
    assert_eq!(result.tool_name, "mock_rustfmt");

    // Run tools for Python language (should be filtered out since we're passing Rust files)
    let results = executor
        .run_tools_for_language(Language::Python, &rust_files, &config)
        .await;

    // We still get a result but it will have no issues because the files don't match
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_executor_run_tools_for_language_and_type() {
    // Create a registry and executor
    let mut registry = DefaultToolRegistry::new();

    // Register tools of different types
    let rust_formatter = MockTool::new(
        "mock_rustfmt",
        Language::Rust,
        ToolType::Formatter,
        "A mock Rust formatter",
        true,
        Some("1.0.0".to_string()),
        0,
    );

    let rust_linter = MockTool::new(
        "mock_clippy",
        Language::Rust,
        ToolType::Linter,
        "A mock Rust linter",
        true,
        Some("0.9.0".to_string()),
        0,
    );

    registry.register_tool(rust_formatter);
    registry.register_tool(rust_linter);

    // Create a dummy ToolConfig
    let config = ToolConfig {
        enabled: true,
        extra_args: Vec::new(),
        env_vars: HashMap::new(),
        executable_path: None,
        report_level: None,
        auto_fix: false,
        check: false,
    };

    // Create dummy file paths
    let rust_files = vec![PathBuf::from("test.rs")];

    // Create executor
    let executor = ToolExecutor::new(registry);

    // Run formatters for Rust language
    let results = executor
        .run_tools_for_language_and_type(Language::Rust, ToolType::Formatter, &rust_files, &config)
        .await;

    // Check that we got one result for the one Rust formatter
    assert_eq!(results.len(), 1);
    assert!(results[0].is_ok());

    // Check the result details
    let result = &results[0].as_ref().unwrap();
    assert_eq!(result.tool_name, "mock_rustfmt");

    // Run linters for Rust language
    let results = executor
        .run_tools_for_language_and_type(Language::Rust, ToolType::Linter, &rust_files, &config)
        .await;

    // Check that we got one result for the one Rust linter
    assert_eq!(results.len(), 1);
    assert!(results[0].is_ok());

    // Check the result details
    let result = &results[0].as_ref().unwrap();
    assert_eq!(result.tool_name, "mock_clippy");
}
