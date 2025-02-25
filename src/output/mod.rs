//! Output formatting for Siren

pub mod terminal;

use crate::config::OutputConfig;
use crate::models::{IssueSeverity, Language, LintResult, ProjectInfo, ToolType};
use colored::Colorize;
use log::debug;
use terminal::{divider, language_emoji, tool_emoji};

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

    // Helper to get language emoji with fallback for no-emoji mode
    fn get_language_emoji(&self, language: &Language) -> &'static str {
        if self.use_emoji {
            language_emoji(language)
        } else {
            ""
        }
    }

    // Helper to get tool emoji with fallback for no-emoji mode
    fn get_tool_emoji(&self, tool_type: &ToolType) -> &'static str {
        if self.use_emoji {
            tool_emoji(tool_type)
        } else {
            ""
        }
    }
}

impl OutputFormatter for PrettyFormatter {
    fn format_detection(&self, project_info: &ProjectInfo) -> String {
        let mut output = String::new();

        // Add enchanted header
        output.push_str("✨ Siren detected the following in your project:\n");

        // Format language info
        let mut language_info = Vec::new();
        for language in &project_info.languages {
            let emoji = self.get_language_emoji(language);
            let name = format!("{:?}", language);
            let files_count = format!(
                "{} files",
                project_info.file_counts.get(language).unwrap_or(&0)
            );

            language_info.push(format!("{} {:12} │ 📂 {:10}", emoji, name, files_count));
        }

        output.push_str(&language_info.join("\n"));
        output.push('\n');

        output
    }

    fn format_results(&self, results: &[LintResult], _config: &OutputConfig) -> String {
        debug!("Formatting {} lint results", results.len());

        let mut output = String::new();

        // Add total issues found
        let total_issues: usize = results.iter().map(|r| r.issues.len()).sum();

        if !results.is_empty() {
            output.push_str(&format!(
                "\nFound {} issues across {} tools\n\n",
                total_issues,
                results.len()
            ));
            output.push_str(&divider());
        }

        // Group results by tool
        for result in results {
            // Get tool information
            let language = result
                .tool
                .as_ref()
                .map(|t| &t.language)
                .unwrap_or(&Language::Unknown);
            let tool_type = result
                .tool
                .as_ref()
                .map(|t| &t.tool_type)
                .unwrap_or(&ToolType::Linter);
            let version = result
                .tool
                .as_ref()
                .and_then(|t| t.version.as_ref())
                .map_or_else(|| "unknown version".to_string(), |v| v.clone());

            // Add tool header with emoji, name, and version
            let language_emoji = self.get_language_emoji(language);
            let tool_emoji = self.get_tool_emoji(tool_type);
            let tool_status = if result.issues.is_empty() {
                "✓".green()
            } else {
                "⚠️".yellow()
            };

            // Format the header with tool info - simplified to avoid redundancy
            output.push_str(&format!(
                "\n\n{}{}{} {} ({})\n",
                language_emoji,
                tool_emoji,
                result.tool_name.bold(),
                tool_status,
                version.dimmed(),
            ));

            output.push_str(&divider());
            output.push_str("\n\n");

            // If no issues, add a success message
            if result.issues.is_empty() {
                output.push_str(&format!("{}\n\n", "All checks passed!".green()));
                continue;
            }

            // Group issues by severity
            let errors = result
                .issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Error)
                .collect::<Vec<_>>();
            let warnings = result
                .issues
                .iter()
                .filter(|i| {
                    i.severity == IssueSeverity::Warning || i.severity == IssueSeverity::Style
                })
                .collect::<Vec<_>>();
            let info = result
                .issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Info)
                .collect::<Vec<_>>();

            // Add issue count summary
            if !errors.is_empty() || !warnings.is_empty() || !info.is_empty() {
                let mut summary = Vec::new();
                if !errors.is_empty() {
                    summary.push(format!("{} errors", errors.len()));
                }
                if !warnings.is_empty() {
                    summary.push(format!("{} style issues", warnings.len()));
                }
                if !info.is_empty() {
                    summary.push(format!("{} info", info.len()));
                }

                output.push_str(&format!("Issues found: {}\n\n", summary.join(", ")));
            }

            // Add the actual issues
            for issue in &result.issues {
                // Format severity
                let severity_str = match issue.severity {
                    IssueSeverity::Error => "error".red(),
                    IssueSeverity::Warning => "style".yellow(),
                    IssueSeverity::Style => "style".magenta(),
                    IssueSeverity::Info => "info".blue(),
                };

                // Format location
                let location = if let Some(filepath) = &issue.file {
                    if let Some(line) = issue.line {
                        if let Some(column) = issue.column {
                            format!("{}:{}:{}", filepath.display(), line, column)
                        } else {
                            format!("{}:{}", filepath.display(), line)
                        }
                    } else {
                        filepath.display().to_string()
                    }
                } else {
                    "".to_string()
                };

                // Format message
                let message = &issue.message;

                // Format rule if present (using code as rule_id)
                let rule = if let Some(code) = &issue.code {
                    format!(" [{}]", code.dimmed())
                } else {
                    "".to_string()
                };

                // Add to output
                if !location.is_empty() {
                    output.push_str(&format!(
                        "{}: {}: {}{}\n",
                        location, severity_str, message, rule
                    ));
                } else {
                    output.push_str(&format!("{}: {}{}\n", severity_str, message, rule));
                }
            }
        }

        output
    }

    fn format_summary(&self, results: &[LintResult]) -> String {
        // Create counters
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut style_count = 0;
        let mut info_count = 0;
        let mut files_affected = std::collections::HashSet::new();
        let mut tool_counts = std::collections::HashMap::new();

        // Count issues
        for result in results {
            for issue in &result.issues {
                match issue.severity {
                    IssueSeverity::Error => error_count += 1,
                    IssueSeverity::Warning => warning_count += 1,
                    IssueSeverity::Style => style_count += 1,
                    IssueSeverity::Info => info_count += 1,
                }

                // Count affected files
                if let Some(filepath) = &issue.file {
                    files_affected.insert(filepath.clone());
                }

                // Count issues by tool
                *tool_counts.entry(result.tool_name.clone()).or_insert(0) += 1;
            }
        }

        // Create result string
        let mut output = String::new();

        // Add divider
        output.push('\n');
        output.push_str(&divider());

        // Add status header
        if error_count > 0 {
            output.push_str(&format!(
                "\n\n  {} {}\n\n",
                "❌".red(),
                "Errors found".red().bold()
            ));
        } else if warning_count > 0 || style_count > 0 {
            output.push_str(&format!(
                "\n\n  {} {}\n\n",
                "⚠️".yellow(),
                "Warnings found".yellow().bold()
            ));
        } else {
            output.push_str(&format!(
                "\n\n  {} {}\n\n",
                "✓".green(),
                "All checks passed!".green().bold()
            ));
        }

        // Add count summary
        output.push_str(&format!(
            "  📊 Found: {} errors, {} style issues, {} info notes\n",
            error_count,
            warning_count + style_count,
            info_count
        ));

        // Add files affected
        output.push_str(&format!("  📁 Affected: {} files\n", files_affected.len()));

        // Add breakdown by tool if there are multiple tools
        if tool_counts.len() > 1 {
            output.push_str("\n  🔍 Breakdown by tool:\n");

            // Sort tools by issue count (descending)
            let mut tools: Vec<_> = tool_counts.iter().collect();
            tools.sort_by(|a, b| b.1.cmp(a.1));

            // Calculate percentages
            let total_issues = error_count + warning_count + style_count + info_count;
            let percentage = |count: &i32| {
                if total_issues > 0 {
                    ((*count as f64 / total_issues as f64) * 100.0).round() as i32
                } else {
                    0
                }
            };

            // Format each tool's breakdown
            for (tool_name, count) in tools {
                let tool_emoji = match tool_name.as_str() {
                    "pylint" => "🐍🔍",
                    "mypy" => "🐍🔎",
                    "ruff" => "🐍🔍",
                    "eslint" => "🌐🔍",
                    "clippy" => "🦀🔍",
                    "rustfmt" => "🦀💅",
                    _ => "🔧",
                };

                output.push_str(&format!(
                    "    {} {} - {} issues ({}%)\n",
                    tool_emoji,
                    tool_name,
                    count,
                    percentage(count)
                ));
            }
        }

        // Add divider at the end
        output.push('\n');
        output.push_str(&divider());

        output
    }
}

/// JSON formatter
#[derive(Clone)]
pub struct JsonFormatter;

impl JsonFormatter {
    /// Create a new JsonFormatter
    fn _new() -> Self {
        Self {}
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
        // Count issues by severity
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut style_count = 0;
        let mut info_count = 0;

        for result in results {
            for issue in &result.issues {
                match issue.severity {
                    IssueSeverity::Error => error_count += 1,
                    IssueSeverity::Warning => warning_count += 1,
                    IssueSeverity::Style => style_count += 1,
                    IssueSeverity::Info => info_count += 1,
                }
            }
        }

        // Create a simple summary object
        let summary = serde_json::json!({
            "total_issues": error_count + warning_count + style_count + info_count,
            "errors": error_count,
            "warnings": warning_count,
            "style": style_count,
            "info": info_count,
            "tools_run": results.len(),
        });

        serde_json::to_string_pretty(&summary).unwrap_or_else(|_| "{}".to_string())
    }
}
