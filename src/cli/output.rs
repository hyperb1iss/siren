use std::collections::HashMap;
use std::fmt;

use colored::*;
use console::Term;
use indicatif::{ProgressBar, ProgressStyle};

use crate::models::{Framework, IssueSeverity, Language, LintIssue, LintResult, ProjectInfo};
use crate::tools::ToolInfo;

/// Output theme
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Theme {
    /// Default theme
    Default,

    /// Enchantress theme (pink/purple based)
    Enchantress,

    /// Ocean theme (blue based)
    Ocean,

    /// Forest theme (green based)
    Forest,

    /// Dark theme
    Dark,

    /// Light theme
    Light,

    /// No colors
    NoColor,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Default
    }
}

impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Default => "default",
            Self::Enchantress => "enchantress",
            Self::Ocean => "ocean",
            Self::Forest => "forest",
            Self::Dark => "dark",
            Self::Light => "light",
            Self::NoColor => "no-color",
        };
        write!(f, "{}", name)
    }
}

impl Theme {
    /// Create a theme from a name
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "default" => Some(Self::Default),
            "enchantress" => Some(Self::Enchantress),
            "ocean" => Some(Self::Ocean),
            "forest" => Some(Self::Forest),
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            "no-color" | "nocolor" => Some(Self::NoColor),
            _ => None,
        }
    }

    /// Get available theme names
    pub fn available_themes() -> Vec<&'static str> {
        vec![
            "default",
            "enchantress",
            "ocean",
            "forest",
            "dark",
            "light",
            "no-color",
        ]
    }
}

/// Output formatter
pub struct SirenOutput {
    /// Terminal output
    terminal: Term,

    /// Current theme
    theme: Theme,

    /// Whether to use emoji in output
    use_emoji: bool,

    /// Whether to use Unicode box drawing characters
    use_unicode: bool,

    /// Whether to use color in output
    use_color: bool,

    /// Current progress bars
    progress_bars: Vec<ProgressBar>,
}

impl SirenOutput {
    /// Create a new output formatter
    pub fn new() -> Self {
        Self {
            terminal: Term::stdout(),
            theme: Theme::default(),
            use_emoji: true,
            use_unicode: true,
            use_color: true,
            progress_bars: Vec::new(),
        }
    }

    /// Set the theme
    pub fn with_theme(mut self, theme: Theme) -> Self {
        let theme_copy = theme.clone();
        self.theme = theme_copy;
        if self.theme == Theme::NoColor {
            self.use_color = false;
        }
        self
    }

    /// Set whether to use emoji
    pub fn with_emoji(mut self, use_emoji: bool) -> Self {
        self.use_emoji = use_emoji;
        self
    }

    /// Set whether to use Unicode box drawing characters
    pub fn with_unicode(mut self, use_unicode: bool) -> Self {
        self.use_unicode = use_unicode;
        self
    }

    /// Set whether to use color
    pub fn with_color(mut self, use_color: bool) -> Self {
        self.use_color = use_color;
        self
    }

    /// Print a formatted header
    pub fn print_header(&self, text: &str) {
        let width = self.terminal.size().1 as usize;
        let text = format!(" {} ", text);
        let padding = width.saturating_sub(text.len()) / 2;
        let line = "═".repeat(padding);

        let header = match self.theme {
            Theme::Enchantress => format!(
                "{}{}{}{}{}",
                "╔".magenta().bold(),
                line.magenta().bold(),
                text.bright_magenta().bold(),
                line.magenta().bold(),
                "╗".magenta().bold()
            ),
            Theme::Ocean => format!(
                "{}{}{}{}{}",
                "╔".blue().bold(),
                line.blue().bold(),
                text.bright_blue().bold(),
                line.blue().bold(),
                "╗".blue().bold()
            ),
            _ => format!(
                "{}{}{}{}{}",
                "╔".cyan().bold(),
                line.cyan().bold(),
                text.bright_cyan().bold(),
                line.cyan().bold(),
                "╗".cyan().bold()
            ),
        };

        println!("{}", header);
    }

    /// Print a formatted footer
    pub fn print_footer(&self) {
        let width = self.terminal.size().1 as usize;
        let line = "═".repeat(width.saturating_sub(2));

        let footer = match self.theme {
            Theme::Enchantress => format!(
                "{}{}{}",
                "╚".magenta().bold(),
                line.magenta().bold(),
                "╝".magenta().bold()
            ),
            Theme::Ocean => format!(
                "{}{}{}",
                "╚".blue().bold(),
                line.blue().bold(),
                "╝".blue().bold()
            ),
            _ => format!(
                "{}{}{}",
                "╚".cyan().bold(),
                line.cyan().bold(),
                "╝".cyan().bold()
            ),
        };

        println!("{}", footer);
    }

    /// Print a section header
    pub fn print_section(&self, text: &str) {
        let formatted = match self.theme {
            Theme::Enchantress => format!("┌─ {} ───", text).bright_magenta().bold(),
            Theme::Ocean => format!("┌─ {} ───", text).bright_blue().bold(),
            _ => format!("┌─ {} ───", text).bright_cyan().bold(),
        };

        println!("\n{}", formatted);
    }

    /// Format a language with emoji
    pub fn format_language(&self, language: Language) -> String {
        let name = format!("{}", language);

        if self.use_emoji {
            format!("{} {}", language.emoji(), name)
        } else {
            name
        }
    }

    /// Format a framework with emoji
    pub fn format_framework(&self, framework: Framework) -> String {
        let name = format!("{}", framework);

        if self.use_emoji {
            format!("{} {}", framework.emoji(), name)
        } else {
            name
        }
    }

    /// Format an issue severity with color
    pub fn format_severity(&self, severity: IssueSeverity) -> ColoredString {
        let text = if self.use_emoji {
            format!("{} {}", severity.emoji(), severity)
        } else {
            format!("{}", severity)
        };

        if !self.use_color {
            return text.normal();
        }

        match severity {
            IssueSeverity::Error => text.red().bold(),
            IssueSeverity::Warning => text.yellow(),
            IssueSeverity::Info => text.blue(),
            IssueSeverity::Style => text.magenta(),
        }
    }

    /// Format a success message
    pub fn format_success(&self, message: &str) -> ColoredString {
        if !self.use_color {
            return message.normal();
        }

        match self.theme {
            Theme::Enchantress => message.bright_magenta(),
            Theme::Ocean => message.bright_blue(),
            _ => message.green(),
        }
    }

    /// Format an error message
    pub fn format_error(&self, message: &str) -> ColoredString {
        if !self.use_color {
            return message.normal();
        }

        message.red().bold()
    }

    /// Format a warning message
    pub fn format_warning(&self, message: &str) -> ColoredString {
        if !self.use_color {
            return message.normal();
        }

        message.yellow()
    }

    /// Format an info message
    pub fn format_info(&self, message: &str) -> ColoredString {
        if !self.use_color {
            return message.normal();
        }

        match self.theme {
            Theme::Enchantress => message.bright_magenta(),
            Theme::Ocean => message.bright_blue(),
            _ => message.cyan(),
        }
    }

    /// Create a new progress bar
    pub fn create_progress_bar(&mut self, len: u64, message: &str) -> ProgressBar {
        let pb = ProgressBar::new(len);

        let style = match self.theme {
            Theme::Enchantress => ProgressStyle::default_bar()
                .template("{spinner:.magenta} {msg} [{bar:40.magenta/magenta}] {pos}/{len}")
                .unwrap()
                .progress_chars("█▇▆▅▄▃▂▁  "),
            _ => ProgressStyle::default_bar()
                .template("{spinner:.cyan} {msg} [{bar:40.cyan/blue}] {pos}/{len}")
                .unwrap()
                .progress_chars("█▇▆▅▄▃▂▁  "),
        };

        pb.set_style(style);
        pb.set_message(message.to_string());

        self.progress_bars.push(pb.clone());
        pb
    }

    /// Print a project info summary
    pub fn print_project_info(&self, info: &ProjectInfo) {
        self.print_header("Project Info");

        // Print languages
        if !info.languages.is_empty() {
            println!("📊 Languages detected:");
            for lang in &info.languages {
                let count = info.file_counts.get(lang).copied().unwrap_or(0);
                println!("  {} - {} files", self.format_language(*lang), count);
            }
        } else {
            println!("No languages detected");
        }

        // Print frameworks
        if !info.frameworks.is_empty() {
            println!("\n🧰 Frameworks detected:");
            for framework in &info.frameworks {
                println!("  {}", self.format_framework(*framework));
            }
        }

        // Print detected tools
        if !info.detected_tools.is_empty() {
            println!("\n🔧 Tools detected:");
            for tool in &info.detected_tools {
                println!("  {} - {}", tool.name, tool.config_path.display());
            }
        }

        self.print_footer();
    }

    /// Print a list of available tools
    pub fn print_available_tools(&self, tools: &[ToolInfo]) {
        self.print_header("Available Tools");

        // Group tools by language
        let mut tools_by_language: HashMap<Language, Vec<&ToolInfo>> = HashMap::new();
        for tool in tools {
            tools_by_language
                .entry(tool.language)
                .or_default()
                .push(tool);
        }

        // Print tools grouped by language
        for (language, language_tools) in tools_by_language {
            println!("\n{}", self.format_language(language));

            // Group by tool type
            let mut tools_by_type: HashMap<_, Vec<_>> = HashMap::new();
            for tool in language_tools {
                tools_by_type.entry(tool.tool_type).or_default().push(tool);
            }

            for (tool_type, type_tools) in tools_by_type {
                println!("  {} Tools:", tool_type);

                for tool in type_tools {
                    let available = if tool.available {
                        "✓".green()
                    } else {
                        "✗".red()
                    };

                    let version = tool.version.as_deref().unwrap_or("unknown");

                    println!(
                        "    {} {} ({}): {}",
                        available, tool.name, version, tool.description
                    );
                }
            }
        }

        self.print_footer();
    }

    /// Print lint results
    pub fn print_lint_results(&self, results: &[LintResult], max_issues_per_tool: usize) {
        self.print_header("Lint Results");

        let mut total_errors = 0;
        let mut total_warnings = 0;
        let mut total_info = 0;
        let mut total_style = 0;

        for result in results {
            let issues_by_severity = result.issues.iter().fold(
                HashMap::<IssueSeverity, Vec<&LintIssue>>::new(),
                |mut acc, issue| {
                    acc.entry(issue.severity).or_default().push(issue);
                    acc
                },
            );

            // Count issues by severity
            let error_count = issues_by_severity
                .get(&IssueSeverity::Error)
                .map_or(0, |v| v.len());
            let warning_count = issues_by_severity
                .get(&IssueSeverity::Warning)
                .map_or(0, |v| v.len());
            let info_count = issues_by_severity
                .get(&IssueSeverity::Info)
                .map_or(0, |v| v.len());
            let style_count = issues_by_severity
                .get(&IssueSeverity::Style)
                .map_or(0, |v| v.len());

            total_errors += error_count;
            total_warnings += warning_count;
            total_info += info_count;
            total_style += style_count;

            // Format tool name and status
            let tool_header = if result.success {
                format!("{} {}", result.tool_name, "✓".green())
            } else {
                format!("{} {}", result.tool_name, "✗".red())
            };

            println!("\n{}", tool_header.bold());

            // If there are no issues, print a success message
            if result.issues.is_empty() {
                println!("  {}", self.format_success("No issues found!"));
                continue;
            }

            // Print issues by severity
            for severity in [
                IssueSeverity::Error,
                IssueSeverity::Warning,
                IssueSeverity::Info,
                IssueSeverity::Style,
            ] {
                if let Some(issues) = issues_by_severity.get(&severity) {
                    if issues.is_empty() {
                        continue;
                    }

                    println!("  {} issues:", self.format_severity(severity));

                    // Print a limited number of issues
                    let display_count = issues.len().min(max_issues_per_tool);
                    for issue in issues.iter().take(display_count) {
                        // Format file and line info
                        let location = if let (Some(file), Some(line)) = (&issue.file, issue.line) {
                            format!("{}:{}", file.display(), line)
                        } else if let Some(file) = &issue.file {
                            format!("{}", file.display())
                        } else {
                            "unknown location".to_string()
                        };

                        // Format code if available
                        let code = if let Some(code) = &issue.code {
                            format!("[{}] ", code)
                        } else {
                            "".to_string()
                        };

                        // Format fix available
                        let fix = if issue.fix_available {
                            " (fix available)"
                        } else {
                            ""
                        };

                        println!(
                            "    {} {}{}{}: {}",
                            location.blue(),
                            code.yellow(),
                            issue.message,
                            fix,
                            severity.emoji()
                        );
                    }

                    // Show how many more issues there are
                    if issues.len() > max_issues_per_tool {
                        let remaining = issues.len() - max_issues_per_tool;
                        println!("    ... and {} more issues", remaining);
                    }
                }
            }
        }

        // Print summary
        println!("\n{}", "Summary:".bold());
        if total_errors > 0 {
            println!(
                "  {} errors",
                self.format_error(&format!("{}", total_errors))
            );
        }
        if total_warnings > 0 {
            println!(
                "  {} warnings",
                self.format_warning(&format!("{}", total_warnings))
            );
        }
        if total_info > 0 {
            println!("  {} info", self.format_info(&format!("{}", total_info)));
        }
        if total_style > 0 {
            println!("  {} style", format!("{}", total_style).magenta());
        }

        if total_errors == 0 && total_warnings == 0 && total_info == 0 && total_style == 0 {
            println!("  {}", self.format_success("All checks passed!"));
        }

        self.print_footer();
    }
}
