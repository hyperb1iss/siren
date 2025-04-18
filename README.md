# 🧜‍♀️ SIREN ✨

> _Code quality enchantress with style_

[![Status](https://img.shields.io/badge/status-in_development-blue?logo=github&logoColor=white)](https://github.com/hyperb1iss/siren)
[![Language](https://img.shields.io/badge/built_with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue?logo=apache)](LICENSE)
[![Tools](https://img.shields.io/badge/supported_tools-15%2B-purple?logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCIgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjZmZmIiBzdHJva2Utd2lkdGg9IjIiPjxwYXRoIGQ9Ik0xMiAyQzYuNDggMiAyIDYuNDggMiAxMnM0LjQ4IDEwIDEwIDEwIDEwLTQuNDggMTAtMTBTMTcuNTIgMiAxMiAyem0xIDE1aC0ydi0yaDJ2MnptMC00aC0yVjdoMnY2eiIvPjwvc3ZnPg==)](README.md#-supported-languages--tools)
[![Languages](https://img.shields.io/badge/languages-5%2B-green?logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCIgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjZmZmIiBzdHJva2Utd2lkdGg9IjIiPjxwYXRoIGQ9Ik0xMCAyMGw0LTE2bTQgMTJsNy03LTctN00zIDE2bDctNy03LTciLz48L3N2Zz4=)](README.md#-supported-languages--tools)

Siren is a powerful frontend for multiple linting tools that makes maintaining code quality a delightful experience. Inspired by the mythological sirens, Siren draws developers in with beautiful vibrant output, smart defaults, and an intuitive interface - making code quality standards irresistible to adopt.

<p align="center">
  <img src="https://via.placeholder.com/800x300/8A2BE2/FFFFFF?text=SIREN+Quality+Guardian" alt="Siren Demo" />
</p>

## ✨ Core Features

- 🌈 **Multi-language Support** - Currently works with Rust, Python, JavaScript/TypeScript and HTML/Templates
- 🔍 **Framework Detection** - Automatically identifies project types and frameworks
- 🧙‍♀️ **Smart Tool Selection** - Chooses the right linters based on detected technologies
- 🪄 **Unified Interface** - One command to rule all your linting needs
- 💅 **Vibrant Output** - Colorful, stylish terminal experience
- ⚙️ **Configuration Flexibility** - Smart defaults with extensive customization options
- ⚡ **High Performance** - Lightning-fast execution with Rust's efficiency
- 🔧 **Auto-fixing** - Automatically resolves common issues when possible
- 🔄 **Git Integration** - Focuses on recently modified files for efficient workflows
- 📊 **Interactive Progress** - Live-updating spinners show the status of each tool

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

# Format and fix in one command
siren format-fix src/
```

## 💖 Currently Supported Languages & Tools

Siren supports a growing collection of languages and tools, automatically selecting the best options for your project.

| Language          | Formatting              | Linting                | Type Checking | Fixing                  |
| ----------------- | ----------------------- | ---------------------- | ------------- | ----------------------- |
| 🦀 Rust           | `rustfmt`               | `clippy`               | -             | `clippy --fix`          |
| 🐍 Python         | `black`, `ruff format`  | `pylint`, `ruff check` | `mypy`        | `ruff --fix`            |
| 🌐 JavaScript     | `prettier`              | `eslint`               | -             | `eslint --fix`          |
| 📘 TypeScript     | `prettier`              | `eslint`               | -             | `eslint --fix`          |
| 🖥️ HTML/Templates | `djlint`                | `djlint`               | -             | `djlint --reformat`     |

### Coming Soon

- 🎨 CSS/SCSS: `prettier`, `stylelint`
- 🐹 Go: `gofmt`, `golangci-lint`
- 💎 Ruby: `rubocop`, `sorbet`
- 📝 Markdown: `prettier`, `markdownlint`
- And many more...

## ⚙️ Configuration: Opinionated But Flexible

Siren believes in "convention over configuration" but respects your preferences.

### Philosophy

- **Works Out-of-box** - Zero config needed for common projects
- **Progressive Configuration** - Add settings only when you need to customize
- **Sensible Defaults** - We make the hard choices so you don't have to
- **Override Anything** - But you can always do it your way

### Configuration File

Siren uses TOML for configuration. Place a `.siren.toml` file in your project root:

```toml
# .siren.toml example

[general]
fail_level = "error"
use_relative_paths = true

[languages.python] 
line_length = 100
ignore_rules = ["E203", "W503"]

[tools.eslint]
extra_args = ["--max-warnings", "10"] 
auto_fix = true
```

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
$ siren format-fix --git-modified
```

### For Team Leads

```bash
# Run comprehensive checks before a release
$ siren check --strict

# Integrate with CI pipeline
$ siren check --ci --fail-level=error

# Create a team config (coming soon)
$ siren init --team
```

### For Newcomers

```bash
# See what Siren detects in your project
$ siren detect

# Learn what tools are available
$ siren list-tools

# Get suggestions for improving code quality (coming soon)
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
    run: siren check --ci --fail-level=error
```

## 🌊 Development Status

Siren is currently in active development. Check the [Project Checklist](docs/CHECKLIST.md) for the current status and roadmap.

The following features are implemented and working:
- ✅ Core architecture and command structure
- ✅ Tool registry and plugin system
- ✅ Support for Rust, Python, and HTML tools
- ✅ Partial support for JavaScript/TypeScript tools
- ✅ Project detection and file collection
- ✅ Basic configuration loading and defaults
- ✅ Colorful terminal output

Coming soon:
- 🔄 Enhanced reporting (HTML, CI annotations)
- 🔄 Additional language support
- 🔄 More comprehensive documentation
- 🔄 Configuration validation and wizards
- 🔄 Performance optimizations

## 🤝 Contributing

Contributions are what make the open source community such a vibrant place! Any contributions you make are **greatly appreciated**.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AwesomeFeature`)
3. Commit your changes (`git commit -m 'Add some AwesomeFeature'`)
4. Push to the branch (`git push origin feature/AwesomeFeature`)
5. Open a Pull Request

Contributing guide coming soon.

## 📜 License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

---

<div align="center">

Created by [Stefanie Jane 🌠](https://github.com/hyperb1iss)

If you find Siren useful, [buy me a Monster Ultra Violet](https://ko-fi.com/hyperb1iss)! ⚡️

</div>
