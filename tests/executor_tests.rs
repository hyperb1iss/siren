use std::collections::HashMap;
use std::path::PathBuf;

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
            use siren::models::{IssueSeverity, LintIssue};

            // Return a mock result with one issue
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

#[tokio::test]
async fn test_executor_run_tool() {
    // Create a mock tool
    let tool = MockTool::new(
        "test-tool",
        vec![Language::Rust],
        ToolType::Linter,
        "Test tool for testing",
        true,
        Some("1.0.0".to_string()),
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
    let runner = ToolRunner::new();
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
        vec![Language::Rust],
        ToolType::Linter,
        "Rust tool 1",
        true,
        Some("1.0.0".to_string()),
    );
    let rust_tool2 = MockTool::new(
        "rust-tool2",
        vec![Language::Rust],
        ToolType::Formatter,
        "Rust tool 2",
        true,
        Some("1.0.0".to_string()),
    );
    let python_tool = MockTool::new(
        "python-tool",
        vec![Language::Python],
        ToolType::Linter,
        "Python tool",
        true,
        Some("1.0.0".to_string()),
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
    let runner = ToolRunner::new();

    // Run tools for Rust language
    // Get all the Rust tools from the registry
    let rust_tools = registry.get_tools_for_language(Language::Rust);
    let results = runner.run_tools(rust_tools, &rust_files, &config).await;

    // Check that we got 2 results (one for each Rust tool)
    assert_eq!(results.len(), 2);

    // Run tools for Python language with Rust files
    // Get all the Python tools from the registry
    let python_tools = registry.get_tools_for_language(Language::Python);
    let results = runner.run_tools(python_tools, &rust_files, &config).await;

    // Check that we got 1 result (for the Python tool)
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_executor_run_tools_for_language_and_type() {
    // Create mock tools
    let rust_linter = MockTool::new(
        "rust-linter",
        vec![Language::Rust],
        ToolType::Linter,
        "Rust linter",
        true,
        Some("1.0.0".to_string()),
    );
    let rust_formatter = MockTool::new(
        "rust-formatter",
        vec![Language::Rust],
        ToolType::Formatter,
        "Rust formatter",
        true,
        Some("1.0.0".to_string()),
    );
    let python_linter = MockTool::new(
        "python-linter",
        vec![Language::Python],
        ToolType::Linter,
        "Python linter",
        true,
        Some("1.0.0".to_string()),
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

    // Get the Rust formatters from the registry
    let rust_formatters =
        registry.get_tools_for_language_and_type(Language::Rust, ToolType::Formatter);
    let formatter_runner = ToolRunner::new();
    let results = formatter_runner
        .run_tools(rust_formatters, &rust_files, &config)
        .await;

    // Check that we got 1 result (for the Rust formatter)
    assert_eq!(results.len(), 1);
    assert!(results[0].is_ok());
    assert_eq!(results[0].as_ref().unwrap().tool_name, "rust-formatter");

    // Get the Rust linters from the registry
    let rust_linters = registry.get_tools_for_language_and_type(Language::Rust, ToolType::Linter);
    let linter_runner = ToolRunner::new();
    let results = linter_runner
        .run_tools(rust_linters, &rust_files, &config)
        .await;

    // Check that we got 1 result (for the Rust linter)
    assert_eq!(results.len(), 1);
    assert!(results[0].is_ok());
    assert_eq!(results[0].as_ref().unwrap().tool_name, "rust-linter");
}
