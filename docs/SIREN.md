# 🧜‍♀️ SIREN

> *Enchanting code quality with irresistible standards*

## ✨ Vision

Siren is a bewitching frontend for multiple linting tools that makes maintaining code quality a delightful experience. Like the mythological sirens that lured sailors with their enchanting voices, Siren entices developers with beautiful output, smart defaults, and an intuitive interface - making code quality standards impossible to resist.

## 🌈 Features

### Core Enchantments

- **Multi-language Support** - Seamlessly works with many programming languages
- **Framework Detection** - Automatically identifies project types and frameworks
- **Smart Tool Selection** - Chooses the right linters based on detected technologies
- **Unified Interface** - One command to rule all your linting needs
- **Beautiful Output** - Colorful, clear, and captivating terminal display
- **Configuration Flexibility** - Smart defaults with extensive customization options
- **High Performance** - Lightning-fast execution with Rust's efficiency
- **Auto-fixing** - Automatically resolves common issues when possible
- **Git Integration** - Focuses on recently modified files for efficient workflows

### Supported Languages & Tools

| Language | Formatting | Linting | Type Checking | Fixing |
|----------|------------|---------|--------------|--------|
| Rust     | `rustfmt`, `cargo fmt` | `clippy` | - | `cargo fix` |
| Python   | `black`, `ruff format` | `pylint`, `ruff check` | `mypy` | `ruff --fix` |
| JavaScript/TypeScript | `prettier`, `dprint` | `eslint` | `typescript` | `eslint --fix` |
| HTML/Templates | `djlint`, `prettier` | `htmlhint` | - | `djlint --reformat` |
| CSS/SCSS | `prettier`, `stylelint` | `stylelint` | - | `stylelint --fix` |
| Go       | `gofmt` | `golangci-lint` | - | `golangci-lint --fix` |
| Ruby     | `rubocop` | `rubocop` | `sorbet` | `rubocop -a` |
| and more to come...

## 💅 Enchanting CLI Experience

Siren's CLI is designed to be a feast for the eyes while providing clear, actionable information. Beauty and function in perfect harmony.

### Visual Design Principles

- **Color Psychology** - Strategic use of colors to convey meaning:
  - 💚 *Green* for successes and passing checks
  - ❤️ *Red* for errors that need attention
  - 💜 *Purple* for Siren's personality and branding
  - 💙 *Blue* for informational messages
  - 💛 *Yellow* for warnings and minor issues

- **Typography & Symbols** - Carefully selected Unicode symbols and emoji to add visual interest and meaning
  - ✓ Clear success indicators
  - ⚠️ Intuitive warning symbols
  - 🔍 Process indicators
  - 💄 Formatting indicators
  - 🧹 Cleanup actions

- **Layout & Structure** - Information organized for maximum clarity
  - Summary boxes for quick understanding
  - Collapsible details for verbose output
  - Progress bars for long-running tasks
  - Categorized issues for easier fixing

### Sample CLI Interactions

```
$ siren

✨ Siren detected the following in your project:
┌─────────────────────────────────────────────┐
│ 🦀 Rust       │ 📂 34 files    │ 🔧 cargo  │
│ 🐍 Python     │ 📂 12 files    │ 🔧 ruff   │
│ 🌐 JavaScript │ 📂 58 files    │ 🔧 eslint │
└─────────────────────────────────────────────┘

🔮 Running checks with optimal settings...

🦀 Rust:
  ✓ Formatting (rustfmt): Perfect! (34 files)
  ✓ Linting (clippy): All good!
  
🐍 Python:
  ✓ Formatting (black): Fabulous! (8 files)
  ⚠️ Linting (ruff): 3 warnings found
    └─ app/models.py:23 - Line too long (88 > 79 characters)
    └─ app/views.py:45 - Unused import 'datetime'
    └─ app/utils.py:12 - Variable name 'x' is too short
  
🌐 JavaScript:
  ❌ Formatting (prettier): 2 files need formatting
    └─ src/components/Header.js
    └─ src/utils/api.js
  ⚠️ Linting (eslint): 5 warnings found
    └─ [Showing top 3, use --verbose for all]
    └─ src/app.js:12 - Unexpected console statement
    └─ src/components/Button.js:34 - Missing prop validation
    └─ src/utils/helpers.js:56 - 'result' is assigned but never used

💫 Summary: 2 errors, 8 warnings

💅 Want to fix formatting issues? Run:
   $ siren format
   
🧹 Want to fix auto-fixable issues? Run:
   $ siren fix
```

## ⚙️ Configuration: Opinionated But Flexible

Siren believes in "convention over configuration" but respects your individuality.

### Philosophy

- **Works Magically Out-of-box** - Zero config needed for common projects
- **Progressive Configuration** - Add settings only when you need to customize
- **Sensible Opinions** - We make the hard choices so you don't have to
- **Override Anything** - But you can always do it your way

### Configuration File

Siren uses the elegant TOML format for configuration. Place a `.siren.toml` file in your project root:

```toml
# .siren.toml - Minimal example with overrides

[general]
# Only fail CI on errors, not warnings
fail_level = "error"
# Show relative paths in reports
use_relative_paths = true

[style]

# Override default settings for Python
[languages.python] 
line_length = 100
ignore_rules = ["E203", "W503"]

# Customize how specific tools run
[tools.eslint]
extra_args = ["--max-warnings", "10"] 
```

### Configuration Cascade

Siren intelligently looks for configuration in multiple places (in order of precedence):

1. **Command-line Arguments** - Flags and options when running Siren
2. **Project Config** - `.siren.toml` in the current directory or parent directories
3. **Framework-specific Configs** - Respects settings in `.eslintrc`, `pyproject.toml`, etc.
4. **User Config** - `~/.config/siren/config.toml` for user preferences
5. **Smart Defaults** - Siren's built-in opinionated defaults

### Project Detection & Auto-Configuration

Siren employs her charms to understand your project:

- Automatically detects project types by scanning files and directories
- Recognizes common frameworks and adapts linting to their conventions
- Identifies existing linter configurations and respects their settings
- Suggests project-appropriate settings when generating config files

## 💎 Use Cases

### The Enchanted Developer

```bash
# Run Siren with no arguments to check the entire project
$ siren

# Format your code beautifully across all languages
$ siren format

# Check just the files you've changed
$ siren --git-modified

# Automatically fix what can be fixed
$ siren fix

# Focus on a specific directory or file
$ siren check src/components/

# Use glob patterns to check specific files or directories
$ siren check . "src/components/**/*.tsx" "lib/**/*.js"

# Fix issues in specific parts of your codebase
$ siren fix core/templates "**/*.html"

# Target a specific language
$ siren check --lang rust

# Chain commands for workflow efficiency
$ siren format fix --git-modified
```

### The Discerning Team Lead

```bash
# Run comprehensive checks before a release
$ siren check --strict

# Generate a beautiful HTML report
$ siren check --report

# Integrate with CI pipeline
$ siren check --ci --fail-level=error

# Create a team config
$ siren init --team
```

### The Curious Newcomer

```bash
# See what Siren detects in your project
$ siren detect

# Learn what tools are available
$ siren list-tools

# Get suggestions for improving code quality
$ siren suggest
```

## 🌊 Implementation Plan

### Phase 1: Core Architecture

- [x] Research existing linting tools and frontends
- [ ] Design the core architecture in Rust
  - [ ] Tool detection subsystem
  - [ ] Configuration management
  - [ ] Plugin architecture for extensibility
  - [ ] Output formatting system
- [ ] Implement basic CLI structure
- [ ] Create project scaffolding and initial documentation

### Phase 2: Language Support - Initial Wave

- [ ] Implement Rust linting integration (`cargo fmt`, `clippy`, `cargo fix`)
- [ ] Implement Python linting integration (`black`, `ruff`, `pylint`, `mypy`)
- [ ] Implement JavaScript/TypeScript integration (`prettier`, `eslint`)
- [ ] Create unified command interface for these languages
- [ ] Develop framework detection for common projects

### Phase 3: Advanced Features

- [ ] Git integration for modified files
- [ ] Automatic fixing capabilities
- [ ] Configuration presets and customization
- [ ] Caching for improved performance
- [ ] Parallel execution for faster linting
- [ ] Terminal UI enhancements

### Phase 4: Expansion & Polish

- [ ] Add support for additional languages
- [ ] Implement HTML report generation
- [ ] Create CI integration helpers
- [ ] Develop team configuration support
- [ ] Design plugin system for community extensions
- [ ] Performance optimizations

### Phase 5: Community & Distribution

- [ ] Package for various package managers
- [ ] Create comprehensive documentation
- [ ] Develop website and demos
- [ ] Set up contribution guidelines
- [ ] Establish release processes

## 💫 Technical Design Highlights

### Architecture

```
siren
├── core/
│   ├── config/        // Configuration management
│   ├── detection/     // Project & tool detection
│   ├── output/        // Display formatting
│   └── runner/        // Tool execution
├── languages/         // Language-specific integrations
├── tools/             // Tool wrappers & interfaces  
├── ui/                // Terminal UI components
└── main.rs            // Entry point
```

### Implementation Challenges

1. **Extensibility** - Designing a plugin system that makes adding new languages and tools straightforward
2. **Configuration** - Creating a configuration system that balances simplicity with power
3. **Detection** - Accurately identifying project types and selecting appropriate tools
4. **Performance** - Ensuring linting remains fast, especially in large projects
5. **Dependencies** - Managing external tool dependencies efficiently

## 🔮 Future Possibilities

- **IDE Integration** - Plugins for VSCode, JetBrains IDEs, etc.
- **Language Server Protocol** - LSP implementation for real-time linting
- **AI-Assisted Fixes** - Suggestions powered by machine learning
- **Custom Rule Creation** - User-defined linting rules
- **Code Metrics** - Quality score and improvement tracking
- **Pre-commit Hooks** - Seamless Git integration

---

*Let Siren's enchanting call guide you to code quality perfection!* 🧜‍♀️✨ 