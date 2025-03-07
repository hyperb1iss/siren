//! Tools and linters supported by Siren

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::errors::ToolError;
use crate::models::{Language, LintResult, ToolConfig, ToolType};

pub mod html;
pub mod javascript;
mod python;
mod rust;

/// Trait for tools that can check code
pub trait LintTool: Send + Sync {
    /// Get the name of the tool
    fn name(&self) -> &str;

    /// Check if this tool can handle the given file
    fn can_handle(&self, file_path: &Path) -> bool;

    /// Execute the tool on the given files
    ///
    /// Implementations MUST follow these guidelines:
    /// 1. Always capture and include the tool's stdout and stderr in the LintResult
    /// 2. Parsing issues into structured data is recommended but optional
    /// 3. Return success=true if the tool executed without errors, even if issues were found
    /// 4. Ensure both issues and raw output are available in the LintResult for better user experience
    ///
    /// See src/tools/python/pylint.rs or src/tools/rust/clippy.rs for reference implementations
    fn execute(&self, files: &[PathBuf], config: &ToolConfig) -> Result<LintResult, ToolError>;

    /// Get the type of tool
    fn tool_type(&self) -> ToolType;

    /// Get the languages this tool works with
    fn languages(&self) -> Vec<Language>;

    /// Get a description of what this tool does
    fn description(&self) -> &str;

    /// Check if the tool is available on the system
    fn is_available(&self) -> bool;

    /// Get the version of the tool
    fn version(&self) -> Option<String>;
}

/// Common functionality for tool implementations
pub struct ToolBase {
    /// Tool name
    pub name: String,

    /// Tool description
    pub description: String,

    /// Tool type
    pub tool_type: ToolType,

    /// Languages this tool is for
    pub languages: Vec<Language>,
}

/// Tool information
#[derive(Clone)]
pub struct ToolInfo {
    /// Tool name
    #[allow(dead_code)]
    pub name: String,

    /// Tool type
    #[allow(dead_code)]
    pub tool_type: ToolType,

    /// Languages this tool is for
    #[allow(dead_code)]
    pub languages: Vec<Language>,

    /// Whether this tool is available on the system
    #[allow(dead_code)]
    pub available: bool,

    /// Version of the tool
    #[allow(dead_code)]
    pub version: Option<String>,

    /// Description of the tool
    #[allow(dead_code)]
    pub description: String,
}

/// Registry of all available tools
pub trait ToolRegistry {
    /// Register a new tool
    fn register_tool(&mut self, tool: Arc<dyn LintTool>);

    /// Get all registered tools
    fn get_all_tools(&self) -> Vec<Arc<dyn LintTool>>;

    /// Get tools for a specific language
    fn get_tools_for_language(&self, language: Language) -> Vec<Arc<dyn LintTool>>;

    /// Get tools of a specific type
    fn get_tools_by_type(&self, tool_type: ToolType) -> Vec<Arc<dyn LintTool>>;

    /// Get a tool by name
    fn get_tool_by_name(&self, name: &str) -> Option<Arc<dyn LintTool>>;

    /// Get tools for a specific language and type
    fn get_tools_for_language_and_type(
        &self,
        language: Language,
        tool_type: ToolType,
    ) -> Vec<Arc<dyn LintTool>>;

    /// Get tool info for all registered tools
    fn get_tool_info(&self) -> Vec<ToolInfo>;
}

/// Define a tool module
#[macro_export]
macro_rules! define_tool_module {
    ($name:ident) => {
        pub mod $name;
    };
}

/// Default implementation of ToolRegistry
#[derive(Clone)]
pub struct DefaultToolRegistry {
    /// Tools by name
    tools: HashMap<String, Arc<dyn LintTool>>,
}

impl Default for DefaultToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultToolRegistry {
    /// Create a new empty DefaultToolRegistry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Create a new DefaultToolRegistry with default tools
    pub fn with_default_tools() -> Self {
        let mut registry = Self::new();

        // Register default Rust tools
        registry.register_tool(Arc::new(rust::Rustfmt::new()));
        registry.register_tool(Arc::new(rust::Clippy::new()));
        registry.register_tool(Arc::new(rust::ClippyFixer::new()));

        // Register Python tools
        registry.register_tool(Arc::new(python::Ruff::new()));
        registry.register_tool(Arc::new(python::PyLint::new()));
        registry.register_tool(Arc::new(python::MyPy::new()));
        registry.register_tool(Arc::new(python::Black::new()));
        registry.register_tool(Arc::new(python::RuffFormatter::new()));

        // Register HTML tools
        registry.register_tool(Arc::new(html::DjLint::new()));
        registry.register_tool(Arc::new(html::DjLintFormatter::new()));

        // Register JavaScript tools
        registry.register_tool(Arc::new(javascript::Prettier::new()));
        registry.register_tool(Arc::new(javascript::ESLint::new()));
        registry.register_tool(Arc::new(javascript::TypeScript::new()));

        registry
    }
}

impl ToolRegistry for DefaultToolRegistry {
    fn register_tool(&mut self, tool: Arc<dyn LintTool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    fn get_all_tools(&self) -> Vec<Arc<dyn LintTool>> {
        let mut tools: Vec<_> = self.tools.values().map(Arc::clone).collect();

        // Sort tools by language and then by name for consistent output
        tools.sort_by(|a, b| {
            // First sort by language
            let a_lang = format!("{:?}", a.languages());
            let b_lang = format!("{:?}", b.languages());

            // Then by name
            a_lang.cmp(&b_lang).then_with(|| a.name().cmp(b.name()))
        });

        tools
    }

    fn get_tools_for_language(&self, lang: Language) -> Vec<Arc<dyn LintTool>> {
        let mut tools: Vec<_> = self
            .tools
            .values()
            .filter(|tool| tool.languages().contains(&lang))
            .map(Arc::clone)
            .collect();

        // Sort tools by name for consistent output
        tools.sort_by(|a, b| a.name().cmp(b.name()));

        tools
    }

    fn get_tools_by_type(&self, tool_type: ToolType) -> Vec<Arc<dyn LintTool>> {
        let mut tools: Vec<_> = self
            .tools
            .values()
            .filter(|tool| tool.tool_type() == tool_type)
            .map(Arc::clone)
            .collect();

        // Sort tools by language and then by name for consistent output
        tools.sort_by(|a, b| {
            // First sort by language
            let a_lang = format!("{:?}", a.languages());
            let b_lang = format!("{:?}", b.languages());

            // Then by name
            a_lang.cmp(&b_lang).then_with(|| a.name().cmp(b.name()))
        });

        tools
    }

    fn get_tool_by_name(&self, name: &str) -> Option<Arc<dyn LintTool>> {
        self.tools.get(name).map(Arc::clone)
    }

    fn get_tools_for_language_and_type(
        &self,
        language: Language,
        tool_type: ToolType,
    ) -> Vec<Arc<dyn LintTool>> {
        let mut tools: Vec<_> = self
            .tools
            .values()
            .filter(|tool| tool.languages().contains(&language) && tool.tool_type() == tool_type)
            .map(Arc::clone)
            .collect();

        // Sort tools by name for consistent output
        tools.sort_by(|a, b| a.name().cmp(b.name()));

        tools
    }

    fn get_tool_info(&self) -> Vec<ToolInfo> {
        let mut tools: Vec<ToolInfo> = self
            .tools
            .values()
            .map(|tool| ToolInfo {
                name: tool.name().to_string(),
                tool_type: tool.tool_type(),
                languages: tool.languages().clone(),
                available: tool.is_available(),
                version: tool.version(),
                description: tool.description().to_string(),
            })
            .collect();

        // Sort tools by language and then by name for consistent output
        tools.sort_by(|a, b| {
            // First sort by language
            let a_lang = format!("{:?}", a.languages);
            let b_lang = format!("{:?}", b.languages);

            // Then by name - use Ord implementation directly to avoid reference issues
            a_lang.cmp(&b_lang).then_with(|| Ord::cmp(&a.name, &b.name))
        });

        tools
    }
}

/// Thread-safe tool registry
pub struct ThreadSafeToolRegistry {
    inner: Arc<RwLock<DefaultToolRegistry>>,
}

impl Default for ThreadSafeToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadSafeToolRegistry {
    /// Create a new thread-safe tool registry
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(DefaultToolRegistry::new())),
        }
    }
}

impl ToolRegistry for ThreadSafeToolRegistry {
    fn register_tool(&mut self, tool: Arc<dyn LintTool>) {
        if let Ok(mut registry) = self.inner.write() {
            registry.register_tool(tool);
        }
    }

    fn get_all_tools(&self) -> Vec<Arc<dyn LintTool>> {
        if let Ok(registry) = self.inner.read() {
            registry.get_all_tools()
        } else {
            Vec::new()
        }
    }

    fn get_tools_for_language(&self, lang: Language) -> Vec<Arc<dyn LintTool>> {
        if let Ok(registry) = self.inner.read() {
            registry.get_tools_for_language(lang)
        } else {
            Vec::new()
        }
    }

    fn get_tools_by_type(&self, tool_type: ToolType) -> Vec<Arc<dyn LintTool>> {
        if let Ok(registry) = self.inner.read() {
            registry.get_tools_by_type(tool_type)
        } else {
            Vec::new()
        }
    }

    fn get_tool_by_name(&self, name: &str) -> Option<Arc<dyn LintTool>> {
        if let Ok(registry) = self.inner.read() {
            registry.get_tool_by_name(name)
        } else {
            None
        }
    }

    fn get_tools_for_language_and_type(
        &self,
        language: Language,
        tool_type: ToolType,
    ) -> Vec<Arc<dyn LintTool>> {
        if let Ok(registry) = self.inner.read() {
            registry.get_tools_for_language_and_type(language, tool_type)
        } else {
            Vec::new()
        }
    }

    fn get_tool_info(&self) -> Vec<ToolInfo> {
        if let Ok(registry) = self.inner.read() {
            registry.get_tool_info()
        } else {
            Vec::new()
        }
    }
}
