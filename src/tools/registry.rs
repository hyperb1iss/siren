use log::{debug, info};
use std::path::Path;
use std::sync::Arc;

use crate::models::{Language, ToolType};
use crate::tools::{LintTool, ToolInfo, ToolRegistry};

/// Default implementation of ToolRegistry
pub struct DefaultToolRegistry {
    /// Tools registered in the registry
    tools: Vec<Arc<dyn LintTool>>,
}

impl DefaultToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// Create a registry with default tools for all languages
    pub fn with_default_tools() -> Self {
        let mut registry = Self::new();
        registry.register_default_tools();
        registry
    }

    /// Check if a tool is already registered
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.iter().any(|t| t.name() == name)
    }

    /// Register all default tools for all languages
    fn register_default_tools(&mut self) {
        // Register Rust tools
        self.register_rust_tools();

        // Register Python tools
        self.register_python_tools();

        // Register JavaScript/TypeScript tools
        self.register_js_tools();

        // Register other tools
        self.register_other_tools();

        info!("Registered {} tools", self.tools.len());
    }

    /// Register Rust tools
    fn register_rust_tools(&mut self) {
        use crate::tools::rust::{Clippy, Rustfmt};

        // Register Rustfmt formatter
        self.register_tool(Arc::new(Rustfmt::new()));

        // Register Clippy linter
        self.register_tool(Arc::new(Clippy::new()));

        debug!("Registered Rust tools");
    }

    /// Register Python tools
    fn register_python_tools(&mut self) {
        use crate::tools::python::{Black, MyPy, PyLint, Ruff};

        // Register Black formatter
        self.register_tool(Arc::new(Black::new()));

        // Register Ruff linter
        self.register_tool(Arc::new(Ruff::new()));

        // Register PyLint linter
        self.register_tool(Arc::new(PyLint::new()));

        // Register MyPy type checker
        self.register_tool(Arc::new(MyPy::new()));

        debug!("Registered Python tools");
    }

    /// Register JavaScript/TypeScript tools
    fn register_js_tools(&mut self) {
        // TODO: Register JavaScript/TypeScript tools when implemented
        debug!("Registered JavaScript/TypeScript tools");
    }

    /// Register other tools
    fn register_other_tools(&mut self) {
        // TODO: Register other tools when implemented
        debug!("Registered other tools");
    }
}

impl ToolRegistry for DefaultToolRegistry {
    fn register_tool(&mut self, tool: Arc<dyn LintTool>) {
        if !self.has_tool(tool.name()) {
            debug!("Registering tool: {}", tool.name());
            self.tools.push(tool);
        } else {
            debug!("Tool already registered: {}", tool.name());
        }
    }

    fn get_all_tools(&self) -> Vec<Arc<dyn LintTool>> {
        self.tools.clone()
    }

    fn get_tools_for_language(&self, language: Language) -> Vec<Arc<dyn LintTool>> {
        self.tools
            .iter()
            .filter(|tool| tool.language() == language)
            .cloned()
            .collect()
    }

    fn get_tools_by_type(&self, tool_type: ToolType) -> Vec<Arc<dyn LintTool>> {
        self.tools
            .iter()
            .filter(|tool| tool.tool_type() == tool_type)
            .cloned()
            .collect()
    }

    fn get_tool_by_name(&self, name: &str) -> Option<Arc<dyn LintTool>> {
        self.tools.iter().find(|tool| tool.name() == name).cloned()
    }

    fn get_tools_for_language_and_type(
        &self,
        language: Language,
        tool_type: ToolType,
    ) -> Vec<Arc<dyn LintTool>> {
        self.tools
            .iter()
            .filter(|tool| tool.language() == language && tool.tool_type() == tool_type)
            .cloned()
            .collect()
    }

    fn get_tool_info(&self) -> Vec<ToolInfo> {
        self.tools
            .iter()
            .map(|tool| ToolInfo {
                name: tool.name().to_string(),
                tool_type: tool.tool_type(),
                language: tool.language(),
                available: tool.is_available(),
                version: tool.version(),
                description: tool.description().to_string(),
            })
            .collect()
    }
}

/// Get tools that can handle a specific file
pub fn get_tools_for_file(registry: &dyn ToolRegistry, file_path: &Path) -> Vec<Arc<dyn LintTool>> {
    registry
        .get_all_tools()
        .into_iter()
        .filter(|tool| tool.can_handle(file_path))
        .collect()
}

/// Get best tool for a file by type
pub fn get_best_tool_for_file(
    registry: &dyn ToolRegistry,
    file_path: &Path,
    tool_type: ToolType,
) -> Option<Arc<dyn LintTool>> {
    registry
        .get_all_tools()
        .into_iter()
        .filter(|tool| tool.can_handle(file_path) && tool.tool_type() == tool_type)
        .max_by_key(|tool| tool.priority())
}
