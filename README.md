# 🧜‍♀️ SIREN ✨

> _Elevating code quality with smart standards_

[![Status](https://img.shields.io/badge/status-in_development-blue?logo=github&logoColor=white)](https://github.com/hyperb1iss/siren)
[![Language](https://img.shields.io/badge/built_with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue?logo=apache)](LICENSE)
[![Tools](https://img.shields.io/badge/supported_tools-30%2B-purple?logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCIgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjZmZmIiBzdHJva2Utd2lkdGg9IjIiPjxwYXRoIGQ9Ik0xMiAyQzYuNDggMiAyIDYuNDggMiAxMnM0LjQ4IDEwIDEwIDEwIDEwLTQuNDggMTAtMTBTMTcuNTIgMiAxMiAyem0xIDE1aC0ydi0yaDJ2MnptMC00aC0yVjdoMnY2eiIvPjwvc3ZnPg==)](README.md#-supported-languages--tools)
[![Languages](https://img.shields.io/badge/languages-7%2B-green?logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCIgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjZmZmIiBzdHJva2Utd2lkdGg9IjIiPjxwYXRoIGQ9Ik0xMCAyMGw0LTE2bTQgMTJsNy03LTctN00zIDE2bDctNy03LTciLz48L3N2Zz4=)](README.md#-supported-languages--tools)

Siren is a powerful frontend for multiple linting tools that makes maintaining code quality a delightful experience. Inspired by the mythological sirens, Siren draws developers in with beautiful output, smart defaults, and an intuitive interface - making code quality standards easy to adopt.

<p align="center">
  <img src="https://via.placeholder.com/800x300/4B0082/FFFFFF?text=SIREN+Demo+Screenshot" alt="Siren Demo" />
</p>

## ✨ Core Features

- 🌈 **Multi-language Support** - Seamlessly works with major programming languages
- 🔍 **Framework Detection** - Automatically identifies project types and frameworks
- 🧙‍♀️ **Smart Tool Selection** - Chooses the right linters based on detected technologies
- 🪄 **Unified Interface** - One command to rule all your linting needs
- 💅 **Beautiful Output** - Colorful, clear, and informative terminal display
- ⚙️ **Configuration Flexibility** - Smart defaults with extensive customization options
- ⚡ **High Performance** - Lightning-fast execution with Rust's efficiency
- 🔧 **Auto-fixing** - Automatically resolves common issues when possible
- 🔄 **Git Integration** - Focuses on recently modified files for efficient workflows

## 🚀 Installation

### Using Cargo (Coming Soon)

```bash
cargo install siren-lint
```

### From Source

```bash
git clone https://github.com/hyperb1iss/siren
cd siren
cargo build --release
```

### Homebrew (Coming Soon)

```bash
brew install hyperb1iss/tap/siren
```

## 🎯 Quick Start

```bash
# Run Siren with no arguments to check the entire project
siren

# Format your code beautifully across all languages
siren format

# Check just the files you've changed
siren --git-modified

# Automatically fix what can be fixed
siren fix

# Focus on a specific directory or file
siren check src/components/

# Target a specific language
siren check --lang rust
```

## 💖 Supported Languages & Tools

Siren supports a growing collection of languages and tools, automatically selecting the best options for your project.

| Language                | Formatting              | Linting                | Type Checking | Fixing                |
| ----------------------- | ----------------------- | ---------------------- | ------------- | --------------------- |
| Rust                    | `rustfmt`, `cargo fmt`  | `clippy`               | -             | `cargo fix`           |
| Python                  | `black`, `ruff format`  | `pylint`, `ruff check` | `mypy`        | `ruff --fix`          |
| JavaScript/TypeScript   | `prettier`, `dprint`    | `eslint`               | `typescript`  | `eslint --fix`        |
| HTML/Templates          | `djlint`, `prettier`    | `htmlhint`             | -             | `djlint --reformat`   |
| CSS/SCSS                | `prettier`, `stylelint` | `stylelint`            | -             | `stylelint --fix`     |
| Go                      | `gofmt`                 | `golangci-lint`        | -             | `golangci-lint --fix` |
| Ruby                    | `rubocop`               | `rubocop`              | `sorbet`      | `rubocop -a`          |
| _and more coming soon!_ |

## 💅 Vibrant CLI Experience

<p align="center">
  <img src="https://via.placeholder.com/750x400/800080/FFFFFF?text=Siren+CLI+Demo" alt="Siren CLI Demo" />
</p>

Siren's CLI is designed to be visually appealing while providing clear, actionable information:

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

### 🎨 Visual Design Principles

- **Color Psychology** - Strategic use of colors to convey meaning:

  - 💚 _Green_ for successes and passing checks
  - ❤️ _Red_ for errors that need attention
  - 💜 _Purple_ for Siren's personality and branding
  - 💙 _Blue_ for informational messages
  - 💛 _Yellow_ for warnings and minor issues

- **Typography & Symbols** - Carefully selected Unicode symbols and emoji to add visual interest and meaning
  - ✓ Clear success indicators
  - ⚠️ Intuitive warning symbols
  - 🔍 Process indicators
  - 💄 Formatting indicators
  - 🧹 Cleanup actions

## ⚙️ Configuration: Opinionated But Flexible

Siren believes in "convention over configuration" but respects your preferences.

### Philosophy

- **Works Out-of-box** - Zero config needed for common projects
- **Progressive Configuration** - Add settings only when you need to customize
- **Sensible Defaults** - We make the hard choices so you don't have to
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
# Use a more feminine color scheme
theme = "enchantress"
# Show emoji in output (default: true)
use_emoji = true

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

## 💎 Advanced Use Cases

### For Developers

```bash
# Format your code beautifully across all languages
$ siren format

# Check just the files you've changed
$ siren --git-modified

# Use glob patterns to check specific files or directories
$ siren check . "src/components/**/*.tsx" "lib/**/*.js"

# Fix issues in specific parts of your codebase
$ siren fix core/templates "**/*.html"

# Chain commands for workflow efficiency
$ siren format fix --git-modified
```

### For Team Leads

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

### For Newcomers

```bash
# See what Siren detects in your project
$ siren detect

# Learn what tools are available
$ siren list-tools

# Get suggestions for improving code quality
$ siren suggest
```

## 🔮 Integration Tips

### Shell Aliases

```bash
# Add to your .bashrc or .zshrc
alias lint="siren"
alias lintfix="siren fix"
alias format="siren format"
alias check="siren --git-modified"
```

### Git Hooks

```bash
# In .git/hooks/pre-commit
siren check --git-staged --fail-level=error
```

### CI/CD Integration

```yaml
# .github/workflows/quality.yml
steps:
  - name: Check code quality
    run: siren check --ci --report --fail-level=error
```

## 🤝 Contributing

Contributions are what make the open source community such a vibrant place! Any contributions you make are **greatly appreciated**.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AwesomeFeature`)
3. Commit your changes (`git commit -m 'Add some AwesomeFeature'`)
4. Push to the branch (`git push origin feature/AwesomeFeature`)
5. Open a Pull Request

See our [Contributing Guidelines](CONTRIBUTING.md) for more information.

## 📜 License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

---

<div align="center">

Created by [Stefanie Jane 🌠](https://github.com/hyperb1iss)

If you find Siren useful, [buy me a coffee](https://ko-fi.com/hyperb1iss)! ☕

</div>
