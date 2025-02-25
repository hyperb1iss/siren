# 🏗️ Siren Architecture

> *Beautiful code beneath the surface*

## 💫 Architectural Principles

Siren follows these core architectural principles to create maintainable, efficient, and elegant code:

1. **Trait-based abstractions** - Use Rust's trait system for clean interfaces
2. **Composition over inheritance** - Build complex behavior through composition
3. **Strong typing** - Leverage Rust's type system for safety and clarity
4. **Error transparency** - Proper error handling with useful context
5. **Immutability by default** - Minimize mutable state
6. **Async where beneficial** - Perform I/O operations asynchronously
7. **Declarative over imperative** - Focus on "what" over "how"
8. **Command pattern** - Encapsulate operations as objects

## 🧩 Core Components

### Component Overview

```
                 ┌─────────┐
                 │   CLI   │
                 └────┬────┘
                      │
                      ▼
┌─────────┐      ┌────────┐     ┌─────────────┐
│ Project │◄────►│  Core  │────►│ Tool Runner │
│ Detector│      │        │     └──────┬──────┘
└─────────┘      │        │            │
                 │        │     ┌──────▼──────┐
┌─────────┐      │        │     │ Tool Plugins│
│ Config  │◄────►│        │     └─────────────┘
│ Provider│      └────────┘
└─────────┘           │
                      │
                 ┌────▼────┐
                 │ Output  │
                 │Formatter│
                 └─────────┘
```

### 1. CLI Component

Responsible for parsing command-line arguments and user interaction.

```rust
// src/cli/mod.rs
pub struct CliOptions {
    pub command: Command,
    pub verbosity: Verbosity,
    pub paths: Vec<PathBuf>,
    pub git_modified_only: bool,
}

pub enum Command {
    Check { strict: bool },
    Format,
    Fix { unsafe_fixes: bool },
    Detect,
    ListTools,
    Init { team: bool },
    // ...
}
```

### 2. Core Application

The central component that orchestrates the workflow.

```rust
// src/app.rs
pub struct SirenApp<D, C, R, O>
where
    D: ProjectDetector,
    C: ConfigProvider,
    R: ToolRegistry,
    O: OutputFormatter,
{
    detector: D,
    config_provider: C,
    tool_registry: R,
    output_formatter: O,
}

impl<D, C, R, O> SirenApp<D, C, R, O>
where
    D: ProjectDetector,
    C: ConfigProvider,
    R: ToolRegistry,
    O: OutputFormatter,
{
    pub fn run(&self, options: CliOptions) -> Result<(), SirenError> {
        // Core application flow
    }
}
```

### 3. Project Detection

Analyzes directories to identify languages, frameworks, and configurations.

```rust
// src/detection/mod.rs
pub trait ProjectDetector {
    fn detect(&self, dir: &Path) -> Result<ProjectInfo, DetectionError>;
}

pub struct ProjectInfo {
    pub languages: Vec<Language>,
    pub frameworks: Vec<Framework>,
    pub file_counts: HashMap<Language, usize>,
    pub detected_tools: Vec<DetectedTool>,
}
```

### 4. Configuration Management

Loads and merges configuration from multiple sources.

```rust
// src/config/mod.rs
pub trait ConfigProvider {
    fn load_config(&self, base_dir: &Path) -> Result<SirenConfig, ConfigError>;
}

pub struct SirenConfig {
    pub general: GeneralConfig,
    pub style: StyleConfig,
    pub languages: HashMap<Language, LanguageConfig>,
    pub tools: HashMap<String, ToolConfig>,
}
```

### 5. Tool Registry and Plugin System

Manages tool discovery, registration, and selection.

```rust
// src/tools/mod.rs
pub trait LintTool: Send + Sync {
    fn name(&self) -> &str;
    fn can_handle(&self, file_path: &Path) -> bool;
    fn execute(&self, files: &[PathBuf], config: &ToolConfig) -> Result<LintResult, ToolError>;
    fn tool_type(&self) -> ToolType;
    fn language(&self) -> Language;
}

pub trait ToolRegistry {
    fn register_tool(&mut self, tool: Box<dyn LintTool>);
    fn get_tools_for_language(&self, lang: Language) -> Vec<&dyn LintTool>;
    fn get_tools_by_type(&self, tool_type: ToolType) -> Vec<&dyn LintTool>;
    fn get_tool_by_name(&self, name: &str) -> Option<&dyn LintTool>;
}
```

### 6. Output Formatting

Transforms lint results into beautiful terminal output.

```rust
// src/output/mod.rs
pub trait OutputFormatter {
    fn format_results(&self, results: &[LintResult], config: &OutputConfig) -> String;
    fn format_detection(&self, project_info: &ProjectInfo) -> String;
    fn format_summary(&self, results: &[LintResult]) -> String;
}
```

### 7. Runner System

Executes linting tools and collects results.

```rust
// src/runner/mod.rs
pub struct ToolRunner<R: ToolRegistry> {
    registry: R,
}

impl<R: ToolRegistry> ToolRunner<R> {
    pub async fn run_tools(
        &self,
        tools: Vec<&dyn LintTool>,
        files: &[PathBuf],
        config: &ToolConfig,
    ) -> Vec<Result<LintResult, ToolError>> {
        // Parallel tool execution
    }
}
```

## 💎 Data Models

### Key Domain Models

```rust
// src/models/mod.rs

// Languages supported by Siren
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Html,
    Css,
    Go,
    Ruby,
    // ...
}

// Types of tools
pub enum ToolType {
    Formatter,
    Linter,
    TypeChecker,
    Fixer,
}

// Framework types
pub enum Framework {
    React,
    Vue,
    Angular,
    Django,
    Flask,
    Rails,
    // ...
}

// Results from running a lint tool
pub struct LintResult {
    pub tool_name: String,
    pub success: bool,
    pub issues: Vec<LintIssue>,
    pub execution_time: Duration,
}

// A specific issue found by a linter
pub struct LintIssue {
    pub severity: IssueSeverity,
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub code: Option<String>,
    pub fix_available: bool,
}

// Severity levels for issues
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
    Style,
}
```

## 🔄 Workflow Example

Here's how the components interact during a typical workflow:

1. **Command Parsing** - CLI parses arguments into a `CliOptions` struct
2. **Project Detection** - `ProjectDetector` scans directories to identify languages and frameworks
3. **Configuration Loading** - `ConfigProvider` loads and merges configurations
4. **Tool Selection** - `ToolRegistry` selects appropriate tools based on detection results
5. **Tool Execution** - `ToolRunner` executes selected tools in parallel
6. **Result Collection** - Results are collected and aggregated
7. **Output Formatting** - `OutputFormatter` transforms results into beautiful terminal output

## 🧬 Implementation Patterns

### 1. Dependency Injection

We'll use constructor injection for flexibility and testability:

```rust
// Example of dependency injection
pub fn build_app() -> SirenApp<impl ProjectDetector, impl ConfigProvider, impl ToolRegistry, impl OutputFormatter> {
    let detector = DefaultProjectDetector::new();
    let config_provider = TomlConfigProvider::new();
    let tool_registry = DefaultToolRegistry::new();
    let output_formatter = ColorfulFormatter::new();
    
    SirenApp::new(detector, config_provider, tool_registry, output_formatter)
}
```

### 2. Error Handling

Clear, context-rich errors with thiserror:

```rust
#[derive(Debug, thiserror::Error)]
pub enum SirenError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("Tool execution error: {0}")]
    Tool(#[from] ToolError),
    
    #[error("Project detection error: {0}")]
    Detection(#[from] DetectionError),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

### 3. Command Pattern

Encapsulate operations as objects:

```rust
pub trait Command {
    fn execute(&self, app: &SirenApp) -> Result<(), SirenError>;
}

pub struct CheckCommand {
    pub paths: Vec<PathBuf>,
    pub git_modified_only: bool,
    pub strict: bool,
}

impl Command for CheckCommand {
    fn execute(&self, app: &SirenApp) -> Result<(), SirenError> {
        // Implementation
    }
}
```

### 4. Builder Pattern

Use builder pattern for complex object construction:

```rust
pub struct SirenConfigBuilder {
    general: Option<GeneralConfig>,
    style: Option<StyleConfig>,
    languages: HashMap<Language, LanguageConfig>,
    tools: HashMap<String, ToolConfig>,
}

impl SirenConfigBuilder {
    pub fn new() -> Self {
        Self {
            general: None,
            style: None,
            languages: HashMap::new(),
            tools: HashMap::new(),
        }
    }
    
    pub fn with_general(mut self, general: GeneralConfig) -> Self {
        self.general = Some(general);
        self
    }
    
    // More builder methods...
    
    pub fn build(self) -> SirenConfig {
        SirenConfig {
            general: self.general.unwrap_or_default(),
            style: self.style.unwrap_or_default(),
            languages: self.languages,
            tools: self.tools,
        }
    }
}
```

## 📁 Directory Structure

```
siren/
├── src/
│   ├── main.rs                 // Entry point
│   ├── app.rs                  // Core application
│   ├── cli/                    // CLI handling
│   │   ├── mod.rs
│   │   ├── args.rs
│   │   ├── commands.rs
│   │   └── output.rs
│   ├── config/                 // Configuration
│   │   ├── mod.rs
│   │   ├── providers.rs
│   │   ├── schema.rs
│   │   └── defaults.rs
│   ├── detection/              // Project detection
│   │   ├── mod.rs
│   │   ├── languages.rs
│   │   ├── frameworks.rs
│   │   └── tools.rs
│   ├── tools/                  // Tool definitions
│   │   ├── mod.rs
│   │   ├── registry.rs
│   │   ├── common.rs
│   │   ├── rust/
│   │   │   ├── mod.rs
│   │   │   ├── rustfmt.rs
│   │   │   └── clippy.rs
│   │   ├── python/
│   │   │   ├── mod.rs
│   │   │   ├── black.rs
│   │   │   └── ruff.rs
│   │   └── javascript/
│   │       ├── mod.rs
│   │       ├── prettier.rs
│   │       └── eslint.rs
│   ├── runner/                 // Execution engine
│   │   ├── mod.rs
│   │   ├── executor.rs
│   │   └── scheduler.rs
│   ├── output/                 // Output formatting
│   │   ├── mod.rs
│   │   ├── formatters.rs
│   │   ├── colors.rs
│   │   └── report.rs
│   ├── models/                 // Data models
│   │   ├── mod.rs
│   │   ├── language.rs
│   │   ├── tools.rs
│   │   └── results.rs
│   └── utils/                  // Utilities
│       ├── mod.rs
│       ├── fs.rs
│       ├── git.rs
│       └── process.rs
├── tests/                      // Integration tests
│   ├── cli_tests.rs
│   ├── detection_tests.rs
│   └── tool_tests.rs
├── Cargo.toml
├── .siren.toml                 // Default configuration
└── README.md
```

## 💫 External Dependencies

Siren will leverage these key Rust crates:

- **clap**: Command line argument parsing
- **serde** + **toml**: Configuration parsing
- **tokio**: Async runtime for parallel tool execution
- **thiserror**: Error handling
- **colored**: Terminal coloring
- **walkdir**: Filesystem traversal
- **globset**: Glob pattern matching
- **regex**: Regular expression support
- **log** + **env_logger**: Logging infrastructure
- **indicatif**: Progress bars and spinners
- **tui** or **crossterm**: Terminal UI elements

## 🧪 Testing Strategy

1. **Unit Tests**: Each component will have comprehensive unit tests
2. **Integration Tests**: Test how components work together
3. **Mock Tools**: Mock linting tools for testing tool execution
4. **Test Fixtures**: Use fixture projects for detection testing
5. **Property-Based Testing**: Use proptest for robust testing of complex logic

## 🚀 Future Extensibility

The architecture enables these future enhancements:

1. **Plugin System**: Allow third-party tools via a plugin interface
2. **Language Server Protocol**: Implement LSP for editor integration
3. **Remote Execution**: Support for running tools on remote machines
4. **Web Interface**: Optional web dashboard for reports
5. **Machine Learning**: Integration for smart suggestions

---

*With this architecture, Siren will be as beautiful on the inside as she is on the outside!* 💖 