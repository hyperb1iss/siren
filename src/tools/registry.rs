use log::debug;
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
        eprintln!("DEBUG: Creating registry with default tools");
        let mut registry = Self::new();
        registry.register_default_tools();
        eprintln!(
            "DEBUG: After registering default tools, total tools: {}",
            registry.tools.len()
        );
        registry
    }

    /// Check if a tool is already registered
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.iter().any(|t| t.name() == name)
    }

    /// Register all default tools for all languages
    fn register_default_tools(&mut self) {
        eprintln!("DEBUG: Registering default tools");
        eprintln!(
            "DEBUG: Before any registration, tool count: {}",
            self.tools.len()
        );

        // Register all tool categories
        self.register_rust_tools();
        eprintln!("DEBUG: After rust tools, tool count: {}", self.tools.len());

        self.register_python_tools(); // Make sure this line is uncommented
        eprintln!(
            "DEBUG: After python tools, tool count: {}",
            self.tools.len()
        );

        self.register_js_tools();
        eprintln!("DEBUG: After js tools, tool count: {}", self.tools.len());

        self.register_other_tools();
        eprintln!("DEBUG: After other tools, tool count: {}", self.tools.len());

        // Debug print all registered tools
        eprintln!("DEBUG: Registered tools:");
        for tool in &self.tools {
            eprintln!("DEBUG:   - {} ({:?})", tool.name(), tool.language());
        }
    }

    /// Register Rust tools
    fn register_rust_tools(&mut self) {
        use crate::tools::rust::{Clippy, ClippyFixer, Rustfmt};

        // Register Rustfmt formatter
        self.register_tool(Arc::new(Rustfmt::new()));

        // Register Clippy linter
        self.register_tool(Arc::new(Clippy::new()));

        // Register ClippyFixer fixer
        self.register_tool(Arc::new(ClippyFixer::new()));

        debug!("Registered Rust tools");
    }

    /// Register Python tools
    fn register_python_tools(&mut self) {
        use crate::tools::python::Black;
        use crate::tools::python::MyPy;
        use crate::tools::python::PyLint;
        use crate::tools::python::Ruff;

        eprintln!("DEBUG: Registering Python tools");

        // Create and check Ruff
        let ruff = Ruff::new();
        let is_ruff_available = ruff.is_available();
        eprintln!("DEBUG: Ruff available: {}", is_ruff_available);
        self.register_tool(Arc::new(ruff));

        // Create and check PyLint
        let pylint = PyLint::new();
        let is_pylint_available = pylint.is_available();
        eprintln!("DEBUG: PyLint available: {}", is_pylint_available);
        self.register_tool(Arc::new(pylint));

        // Create and check MyPy
        let mypy = MyPy::new();
        let is_mypy_available = mypy.is_available();
        eprintln!("DEBUG: MyPy available: {}", is_mypy_available);
        self.register_tool(Arc::new(mypy));

        // Create and check Black
        let black = Black::new();
        let is_black_available = black.is_available();
        eprintln!("DEBUG: Black available: {}", is_black_available);
        self.register_tool(Arc::new(black));

        eprintln!("DEBUG: Python tools registration complete");
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
        eprintln!("DEBUG: Getting tools for language: {:?}", language);
        eprintln!("DEBUG: Total tools in registry: {}", self.tools.len());

        // Debug print all tools first
        eprintln!("DEBUG: All tools in registry:");
        for tool in &self.tools {
            eprintln!(
                "DEBUG:   Tool: {}, Language: {:?} ({:?})",
                tool.name(),
                tool.language(),
                std::mem::discriminant(&tool.language())
            );
        }

        // Debug print language we're looking for
        eprintln!(
            "DEBUG: Looking for language: {:?} ({:?})",
            language,
            std::mem::discriminant(&language)
        );

        let tools = self.tools
            .iter()
            .filter(|tool| {
                let tool_lang = tool.language();
                let matches = tool_lang == language;
                eprintln!("DEBUG:   - Tool: {}, Language: {:?}, Matches: {} (comparison: {:?} == {:?}, or {:?} == {:?})", 
                          tool.name(),
                          tool_lang,
                          matches,
                          tool_lang,
                          language,
                          std::mem::discriminant(&tool_lang),
                          std::mem::discriminant(&language));
                matches
            })
            .cloned()
            .collect::<Vec<_>>();

        eprintln!(
            "DEBUG: Found {} tools for language {:?}",
            tools.len(),
            language
        );
        tools
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
