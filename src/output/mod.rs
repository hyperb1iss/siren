//! Output formatting for Siren

use crate::config::OutputConfig;
use crate::models::{LintResult, ProjectInfo};

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
pub struct PrettyFormatter {
    /// Whether to use emojis
    use_emoji: bool,

    /// Whether to use colors (kept for future use)
    _use_colors: bool,
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
                output.push_str("\n");
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

    fn format_results(&self, results: &[LintResult], config: &OutputConfig) -> String {
        let mut output = String::new();

        // Group results by file path
        let mut by_file: std::collections::HashMap<
            String,
            Vec<(&LintResult, &crate::models::LintIssue)>,
        > = std::collections::HashMap::new();

        // First, collect all issues with their file paths
        for result in results {
            for issue in &result.issues {
                if let Some(file) = &issue.file {
                    // Get filename for grouping
                    let file_key = if config.show_file_paths {
                        file.display().to_string()
                    } else {
                        file.file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_else(|| "unknown".to_string())
                    };

                    // Add to file group
                    by_file.entry(file_key).or_default().push((result, issue));
                } else {
                    // For issues without files, group under tool name
                    by_file
                        .entry(format!("[{}]", result.tool_name))
                        .or_default()
                        .push((result, issue));
                }
            }
        }

        // If there are no issues, just return empty string
        if by_file.is_empty() {
            return output;
        }

        // Sort files for consistent output
        let mut file_keys: Vec<_> = by_file.keys().collect();
        file_keys.sort();

        for file_key in file_keys {
            let issues = &by_file[file_key];

            // Skip empty issue lists
            if issues.is_empty() {
                continue;
            }

            // File header
            output.push_str(&format!("\n📄 {}:\n", file_key));

            // Sort issues by line number
            let mut sorted_issues = issues.clone();
            sorted_issues.sort_by_key(|(_, issue)| issue.line.unwrap_or(0));

            // Track previous line to avoid duplicates
            let mut prev_line: Option<usize> = None;

            for (result, issue) in sorted_issues {
                // Format issue line
                let line_info = if let Some(line) = issue.line {
                    // If we're showing the same line again, use a different prefix
                    if prev_line == Some(line) {
                        format!("      ")
                    } else {
                        prev_line = Some(line);
                        format!("L{:<4}", line)
                    }
                } else {
                    format!("     ")
                };

                let severity_icon = if self.use_emoji {
                    match issue.severity {
                        crate::models::IssueSeverity::Error => "🔴 ",
                        crate::models::IssueSeverity::Warning => "🟠 ",
                        crate::models::IssueSeverity::Info => "🔵 ",
                        crate::models::IssueSeverity::Style => "💜 ",
                    }
                } else {
                    ""
                };

                // Tool name for reference
                let tool_name = result.tool_name.as_str();

                output.push_str(&format!(
                    "  {} {} [{}]: {}\n",
                    line_info, severity_icon, tool_name, issue.message
                ));

                // Show code snippet if available and enabled
                if config.show_code_snippets && issue.code.is_some() {
                    // Use simple indentation
                    let formatted_code = issue
                        .code
                        .as_ref()
                        .unwrap()
                        .lines()
                        .map(|line| format!("       │ {}", line))
                        .collect::<Vec<_>>()
                        .join("\n");

                    output.push_str(&format!(
                        "       ┌───────────────────\n{}\n       └───────────────────\n",
                        formatted_code
                    ));
                }
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

        // Determine success icon
        let status_icon = if self.use_emoji {
            if error_count == 0 && warning_count == 0 {
                "✨"
            } else if error_count == 0 {
                "⚠️"
            } else {
                "❌"
            }
        } else {
            if error_count == 0 && warning_count == 0 {
                "SUCCESS"
            } else if error_count == 0 {
                "WARNINGS"
            } else {
                "FAILED"
            }
        };

        let mut output = format!(
            "{} Summary: {} errors, {} warnings",
            status_icon, error_count, warning_count
        );

        // Add style and info counts if present
        if style_count > 0 || info_count > 0 {
            output.push_str(&format!(
                "\nTotal: {} issues found in {} files",
                total_issues,
                unique_files.len()
            ));

            // Add per-tool breakdown if we have multiple tools with issues
            if tool_issues.len() > 1 {
                output.push_str("\n\nBreakdown by tool:");
                for (tool, count) in tool_issues.iter() {
                    output.push_str(&format!("\n  {}: {} issues", tool, count));
                }
            }
        }

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
