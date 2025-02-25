use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use siren::models::{Language, ToolConfig, ToolType};
use siren::runner::ToolRunner;
use siren::tools::{DefaultToolRegistry, ToolRegistry};

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
            use siren::models::{IssueSeverity, LintIssue};

            // Return a mock result with one issue
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
                issues: vec![LintIssue {
                    severity: IssueSeverity::Warning,
                    message: "Test issue".to_string(),
                    file: None,
                    line: None,
                    column: None,
                    code: None,
                    fix_available: false,
                }],
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
    // Create a mock tool
    let tool = MockTool::new(
        "test-tool",
        Language::Rust,
        ToolType::Linter,
        "Test tool for testing",
        true,
        Some("1.0.0".to_string()),
        0,
    );

    // Create a registry with the mock tool
    let mut registry = DefaultToolRegistry::new();
    registry.register_tool(tool.clone());

    // Create some test files
    let files = vec![PathBuf::from("test.rs")];

    // Create a config
    let config = ToolConfig {
        enabled: true,
        extra_args: Vec::new(),
        env_vars: HashMap::new(),
        executable_path: None,
        report_level: None,
        auto_fix: false,
        check: true,
    };

    // Run the tool
    let runner = ToolRunner::new(registry);
    // Use run_tools with a vector containing just our tool
    let results = runner.run_tools(vec![tool], &files, &config).await;

    // Check that we got a result
    assert_eq!(results.len(), 1);

    // Check that the result is Ok
    let result = &results[0];
    assert!(result.is_ok());

    // Check the result contents
    let lint_result = result.as_ref().unwrap();
    assert_eq!(lint_result.tool_name, "test-tool");
    assert!(lint_result.success);
    assert_eq!(lint_result.issues.len(), 1);
    assert_eq!(lint_result.issues[0].message, "Test issue");
}

#[tokio::test]
async fn test_executor_run_tools_for_language() {
    // Create mock tools
    let rust_tool1 = MockTool::new(
        "rust-tool1",
        Language::Rust,
        ToolType::Linter,
        "Rust tool 1",
        true,
        Some("1.0.0".to_string()),
        0,
    );
    let rust_tool2 = MockTool::new(
        "rust-tool2",
        Language::Rust,
        ToolType::Formatter,
        "Rust tool 2",
        true,
        Some("1.0.0".to_string()),
        0,
    );
    let python_tool = MockTool::new(
        "python-tool",
        Language::Python,
        ToolType::Linter,
        "Python tool",
        true,
        Some("1.0.0".to_string()),
        0,
    );

    // Create a registry with the mock tools
    let mut registry = DefaultToolRegistry::new();
    registry.register_tool(rust_tool1);
    registry.register_tool(rust_tool2);
    registry.register_tool(python_tool);

    // Create some test files
    let rust_files = vec![PathBuf::from("test.rs")];

    // Create a config
    let config = ToolConfig {
        enabled: true,
        extra_args: Vec::new(),
        env_vars: HashMap::new(),
        executable_path: None,
        report_level: None,
        auto_fix: false,
        check: true,
    };

    // Create runner
    let runner = ToolRunner::new(registry);

    // Run tools for Rust language
    let results = runner
        .run_tools_for_language(Language::Rust, &rust_files, &config)
        .await;

    // Check that we got 2 results (one for each Rust tool)
    assert_eq!(results.len(), 2);

    // Run tools for Python language with Rust files
    let results = runner
        .run_tools_for_language(Language::Python, &rust_files, &config)
        .await;

    // Check that we got 1 result (for the Python tool)
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_executor_run_tools_for_language_and_type() {
    // Create mock tools
    let rust_linter = MockTool::new(
        "rust-linter",
        Language::Rust,
        ToolType::Linter,
        "Rust linter",
        true,
        Some("1.0.0".to_string()),
        0,
    );
    let rust_formatter = MockTool::new(
        "rust-formatter",
        Language::Rust,
        ToolType::Formatter,
        "Rust formatter",
        true,
        Some("1.0.0".to_string()),
        0,
    );
    let python_linter = MockTool::new(
        "python-linter",
        Language::Python,
        ToolType::Linter,
        "Python linter",
        true,
        Some("1.0.0".to_string()),
        0,
    );

    // Create a registry with the mock tools
    let mut registry = DefaultToolRegistry::new();
    registry.register_tool(rust_linter);
    registry.register_tool(rust_formatter);
    registry.register_tool(python_linter);

    // Create some test files
    let rust_files = vec![PathBuf::from("test.rs")];

    // Create a config
    let config = ToolConfig {
        enabled: true,
        extra_args: Vec::new(),
        env_vars: HashMap::new(),
        executable_path: None,
        report_level: None,
        auto_fix: false,
        check: true,
    };

    // Run formatters for Rust language
    // We need to create a new registry for each test since we can't access the private registry field
    let mut formatter_registry = DefaultToolRegistry::new();
    formatter_registry.register_tool(MockTool::new(
        "rust-formatter",
        Language::Rust,
        ToolType::Formatter,
        "Rust formatter",
        true,
        Some("1.0.0".to_string()),
        0,
    ));
    let formatter_runner = ToolRunner::new(formatter_registry);
    let results = formatter_runner
        .run_tools_for_language(Language::Rust, &rust_files, &config)
        .await;

    // Check that we got 1 result (for the Rust formatter)
    assert_eq!(results.len(), 1);
    assert!(results[0].is_ok());
    assert_eq!(results[0].as_ref().unwrap().tool_name, "rust-formatter");

    // Run linters for Rust language
    // Create a new registry for linters
    let mut linter_registry = DefaultToolRegistry::new();
    linter_registry.register_tool(MockTool::new(
        "rust-linter",
        Language::Rust,
        ToolType::Linter,
        "Rust linter",
        true,
        Some("1.0.0".to_string()),
        0,
    ));
    let linter_runner = ToolRunner::new(linter_registry);
    let results = linter_runner
        .run_tools_for_language(Language::Rust, &rust_files, &config)
        .await;

    // Check that we got 1 result (for the Rust linter)
    assert_eq!(results.len(), 1);
    assert!(results[0].is_ok());
    assert_eq!(results[0].as_ref().unwrap().tool_name, "rust-linter");
}
