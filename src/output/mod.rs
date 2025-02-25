//! Output formatting for Siren

use crate::config::OutputConfig;
use crate::models::{IssueSeverity, Language, LintResult, ProjectInfo, ToolType};
use colored::Colorize;

/// Trait for formatting output
pub trait OutputFormatter {
    /// Format project detection results
    fn format_detection(&self, project_info: &ProjectInfo) -> String;

    /// Format lint results
    fn format_results(&self, results: &[LintResult], config: &OutputConfig) -> String;

    /// Format a summary of lint results
    fn format_summary(&self, results: &[LintResult]) -> String;
}

/// Default implementation that uses pretty formatting with colors
#[derive(Clone)]
pub struct PrettyFormatter {
    /// Whether to use emojis
    use_emoji: bool,

    /// Whether to use colors (kept for future use)
    _use_colors: bool,
}

impl Default for PrettyFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl PrettyFormatter {
    /// Create a new PrettyFormatter
    pub fn new() -> Self {
        Self {
            use_emoji: true,
            _use_colors: true,
        }
    }

    /// Create a new PrettyFormatter with custom settings (kept for future use)
    fn _with_options(use_emoji: bool, use_colors: bool) -> Self {
        Self {
            use_emoji,
            _use_colors: use_colors,
        }
    }
}

impl OutputFormatter for PrettyFormatter {
    fn format_detection(&self, project_info: &ProjectInfo) -> String {
        let mut output = String::new();

        // Header
        if self.use_emoji {
            output.push_str("✨ Siren detected the following in your project:\n");
        } else {
            output.push_str("Siren detected the following in your project:\n");
        }

        // Languages - no box borders, more compact format
        for lang in &project_info.languages {
            let file_count = project_info.file_counts.get(lang).unwrap_or(&0);

            // Get emoji for language
            let lang_emoji = if self.use_emoji {
                match lang {
                    crate::models::Language::Rust => "🦀 ",
                    crate::models::Language::Python => "🐍 ",
                    crate::models::Language::JavaScript => "🌐 ",
                    crate::models::Language::TypeScript => "📘 ",
                    crate::models::Language::Html => "🌐 ",
                    crate::models::Language::Css => "🎨 ",
                    crate::models::Language::Go => "🐹 ",
                    crate::models::Language::Ruby => "💎 ",
                    crate::models::Language::Java => "☕ ",
                    crate::models::Language::Php => "🐘 ",
                    crate::models::Language::C => "🔍 ",
                    crate::models::Language::Cpp => "🔧 ",
                    crate::models::Language::CSharp => "🔷 ",
                    crate::models::Language::Swift => "🔶 ",
                    crate::models::Language::Markdown => "📝 ",
                    crate::models::Language::Json => "📋 ",
                    crate::models::Language::Yaml => "📄 ",
                    crate::models::Language::Toml => "📁 ",
                    _ => "📄 ",
                }
            } else {
                ""
            };

            let file_emoji = if self.use_emoji { "📂 " } else { "" };

            // Format line - without box borders
            output.push_str(&format!(
                "{}{:<10} │ {}{} files    ",
                lang_emoji,
                format!("{:?}", lang),
                file_emoji,
                file_count
            ));

            // Add detected tools for this language
            let tools: Vec<_> = project_info
                .detected_tools
                .iter()
                .filter(|t| t.language == *lang)
                .collect();

            if !tools.is_empty() {
                let tool_emoji = if self.use_emoji { "🔧 " } else { "" };
                let tool_names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
                output.push_str(&format!("│ {}{}\n", tool_emoji, tool_names.join(", ")));
            } else {
                output.push('\n');
            }
        }

        // Framework info
        if !project_info.frameworks.is_empty() {
            let framework_names: Vec<_> = project_info
                .frameworks
                .iter()
                .map(|f| format!("{:?}", f))
                .collect();

            if self.use_emoji {
                output.push_str(&format!("🧩 Frameworks: {}\n", framework_names.join(", ")));
            } else {
                output.push_str(&format!("Frameworks: {}\n", framework_names.join(", ")));
            }
        }

        output
    }

    fn format_results(&self, results: &[LintResult], _config: &OutputConfig) -> String {
        let mut output = String::new();

        // Display results for each tool
        for result in results {
            // Determine the tool emoji based on language and tool type
            let tool_symbol = match (
                result.tool.as_ref().map(|t| t.language),
                result.tool.as_ref().map(|t| t.tool_type),
            ) {
                (Some(Language::Rust), Some(ToolType::Linter)) => "🦀🔍",
                (Some(Language::Rust), Some(ToolType::Formatter)) => "🦀🎨",
                (Some(Language::Rust), Some(ToolType::TypeChecker)) => "🦀🔎",
                (Some(Language::Rust), Some(ToolType::Fixer)) => "🦀🔧",
                (Some(Language::Python), Some(ToolType::Linter)) => "🐍🔍",
                (Some(Language::Python), Some(ToolType::Formatter)) => "🐍🎨",
                (Some(Language::Python), Some(ToolType::TypeChecker)) => "🐍🔎",
                (Some(Language::Python), Some(ToolType::Fixer)) => "🐍🔧",
                (Some(Language::JavaScript), _) => "🌐",
                (Some(Language::TypeScript), _) => "📘",
                _ => "🔮",
            };

            // Get version info if available
            let version_info = result
                .tool
                .as_ref()
                .and_then(|t| t.version.as_ref())
                .map_or("".to_string(), |v| format!(" ({})", v));

            // Create a header with tool name, status, and version
            let status_icon = if result.success {
                "✓".green()
            } else {
                "✗".red()
            };

            // Add separator before each tool
            output.push('\n');

            // Add a nice separator
            let separator = "━".repeat(60).dimmed();
            output.push_str(&format!("{}\n\n", separator));

            // Tool header with icon, name and version
            let header = format!(
                "{} {} {}{}\n",
                tool_symbol,
                result.tool_name.bold(),
                status_icon,
                version_info
            );

            output.push_str(&header);

            // Add a nice separator
            output.push_str(&format!("{}\n\n", separator));

            // Display issues summary if we have issues
            if !result.issues.is_empty() {
                // Count issues by severity
                let mut error_count = 0;
                let mut warning_count = 0;
                let mut style_count = 0;
                let mut info_count = 0;

                for issue in &result.issues {
                    match issue.severity {
                        IssueSeverity::Error => error_count += 1,
                        IssueSeverity::Warning => warning_count += 1,
                        IssueSeverity::Style => style_count += 1,
                        IssueSeverity::Info => info_count += 1,
                    }
                }

                // Create a summary line with colored counts
                let mut summary_parts = Vec::new();

                if error_count > 0 {
                    summary_parts.push(format!("{} {}", error_count, "errors".red()));
                }

                if warning_count > 0 {
                    summary_parts.push(format!("{} {}", warning_count, "warnings".yellow()));
                }

                if style_count > 0 {
                    summary_parts.push(format!("{} {}", style_count, "style issues".magenta()));
                }

                if info_count > 0 {
                    summary_parts.push(format!("{} {}", info_count, "info".blue()));
                }

                if !summary_parts.is_empty() {
                    output.push_str(&format!("Issues found: {}\n\n", summary_parts.join(", ")));
                }
            }

            // Show the tool's native output if available
            let has_stdout = result.stdout.as_ref().map_or(false, |s| !s.is_empty());
            let has_stderr = result.stderr.as_ref().map_or(false, |s| !s.is_empty());

            if has_stdout || has_stderr {
                if has_stdout {
                    output.push_str(&format!("{}\n", result.stdout.as_ref().unwrap().trim()));
                }

                if has_stderr {
                    // If we already displayed stdout, add some spacing
                    if has_stdout {
                        output.push_str("\n\n");
                    }

                    // If stderr contains an error message, format it nicely
                    let stderr = result.stderr.as_ref().unwrap();
                    if stderr.contains("error:") {
                        output.push_str(&format!("  {}\n", stderr.trim().red()));
                    } else {
                        output.push_str(&format!("  {}\n", stderr.trim()));
                    }
                }

                output.push('\n');
            } else if result.issues.is_empty() {
                // No output and no issues, show a success message
                output.push_str(&format!("  {} No issues detected!\n\n", "✨".green()));
            } else {
                // Issues found but no output captured - this shouldn't happen now that we're fixing all tools
                output.push_str(&format!(
                    "  {} Found {} issues but output was not captured.\n\n",
                    "⚠️".yellow(),
                    result.issues.len()
                ));
            }
        }

        output
    }

    fn format_summary(&self, results: &[LintResult]) -> String {
        // Count errors and warnings
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut style_count = 0;
        let mut info_count = 0;
        let mut total_issues = 0;

        // Count issues by tool
        let mut tool_issues = std::collections::HashMap::new();

        // Count files with issues
        let mut unique_files = std::collections::HashSet::new();

        for result in results {
            for issue in &result.issues {
                match issue.severity {
                    crate::models::IssueSeverity::Error => error_count += 1,
                    crate::models::IssueSeverity::Warning => warning_count += 1,
                    crate::models::IssueSeverity::Style => style_count += 1,
                    crate::models::IssueSeverity::Info => info_count += 1,
                }
                total_issues += 1;

                // Track unique files with issues
                if let Some(file) = &issue.file {
                    unique_files.insert(file.clone());
                }
            }

            // Count issues by tool
            let tool_name = &result.tool_name;
            let issue_count = result.issues.len();

            if issue_count > 0 {
                tool_issues
                    .entry(tool_name.clone())
                    .and_modify(|count| *count += issue_count)
                    .or_insert(issue_count);
            }
        }

        // Determine overall status
        let (status_icon, status_text) =
            if error_count == 0 && warning_count == 0 && style_count == 0 {
                if total_issues == 0 {
                    ("✨", "Perfect! No issues found".green().bold())
                } else {
                    ("✨", "Success! Only informational notes".green().bold())
                }
            } else if error_count == 0 && warning_count == 0 {
                ("🎨", "Good! Only style suggestions".blue().bold())
            } else if error_count == 0 {
                ("⚠️", "Warnings found".yellow().bold())
            } else {
                ("❌", "Errors found".red().bold())
            };

        // Create the summary header with a nice separator
        let separator = "━".repeat(80).dimmed();
        let mut output = format!("\n\n{}\n\n  {} {}\n\n", separator, status_icon, status_text);

        // Create a detailed breakdown with pretty colors
        let mut counts = Vec::new();

        if error_count > 0 {
            counts.push(format!("{} {}", error_count, "errors".red().bold()));
        }

        if warning_count > 0 {
            counts.push(format!("{} {}", warning_count, "warnings".yellow().bold()));
        }

        if style_count > 0 {
            counts.push(format!(
                "{} {}",
                style_count,
                "style issues".magenta().bold()
            ));
        }

        if info_count > 0 {
            counts.push(format!("{} {}", info_count, "info notes".blue().bold()));
        }

        if !counts.is_empty() {
            output.push_str(&format!("  📊 Found: {}\n", counts.join(", ")));
            output.push_str(&format!("  📁 Affected: {} files\n", unique_files.len()));
        }

        // Add per-tool breakdown with nice icons and colors
        if !tool_issues.is_empty() {
            output.push_str("\n  🔍 Breakdown by tool:\n");

            // Sort tools by number of issues (descending)
            let mut tools: Vec<_> = tool_issues.iter().collect();
            tools.sort_by(|a, b| b.1.cmp(a.1));

            for (tool, count) in tools {
                // Get percentage of total issues
                let percentage = (*count as f64 / total_issues as f64 * 100.0).round() as usize;

                // Determine icon based on tool name
                let tool_icon = if tool.contains("ruff") {
                    "🐍🔍"
                } else if tool.contains("pylint") {
                    "🐍📋"
                } else if tool.contains("mypy") {
                    "🐍🔎"
                } else if tool.contains("clippy") {
                    "🦀🔍"
                } else if tool.contains("rustfmt") {
                    "🦀🎨"
                } else if tool.contains("black") {
                    "🐍🎨"
                } else if tool.contains("eslint") {
                    "🌐🔍"
                } else if tool.contains("prettier") {
                    "🌐🎨"
                } else {
                    "🔧"
                };

                // Show tool name, issue count, and percentage
                output.push_str(&format!(
                    "    {} {} - {} issues ({}%)\n",
                    tool_icon,
                    tool.bold(),
                    count,
                    percentage
                ));
            }
        }

        // Add final separator with more space
        output.push_str(&format!("\n{}\n", separator));

        output
    }
}

/// JSON formatter for machine-readable output
pub struct JsonFormatter;

impl JsonFormatter {
    /// Create a new JsonFormatter (kept for future use)
    fn _new() -> Self {
        Self
    }
}

impl OutputFormatter for JsonFormatter {
    fn format_detection(&self, project_info: &ProjectInfo) -> String {
        serde_json::to_string_pretty(project_info).unwrap_or_else(|_| "{}".to_string())
    }

    fn format_results(&self, results: &[LintResult], _config: &OutputConfig) -> String {
        serde_json::to_string_pretty(results).unwrap_or_else(|_| "[]".to_string())
    }

    fn format_summary(&self, results: &[LintResult]) -> String {
        // Count errors and warnings
        let mut error_count = 0;
        let mut warning_count = 0;

        for result in results {
            for issue in &result.issues {
                match issue.severity {
                    crate::models::IssueSeverity::Error => error_count += 1,
                    crate::models::IssueSeverity::Warning => warning_count += 1,
                    _ => {}
                }
            }
        }

        let summary = serde_json::json!({
            "success": error_count == 0,
            "error_count": error_count,
            "warning_count": warning_count,
            "issue_count": error_count + warning_count,
        });

        serde_json::to_string_pretty(&summary).unwrap_or_else(|_| "{}".to_string())
    }
}
