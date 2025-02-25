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

    fn format_results(&self, results: &[LintResult], config: &OutputConfig) -> String {
        let mut output = String::new();

        // Group results by file path
        let mut by_file: std::collections::HashMap<
            String,
            Vec<(&LintResult, &crate::models::LintIssue)>,
        > = std::collections::HashMap::new();

        // First, see if we have any clippy or clippy-fix results with stdout/stderr
        // that we want to show directly for better formatting and context
        let clippy_results: Vec<_> = results
            .iter()
            .filter(|r| r.tool_name == "clippy" || r.tool_name == "clippy-fix")
            .collect();

        if !clippy_results.is_empty() && config.show_code_snippets {
            // Show original clippy output for better context, when available
            for result in clippy_results {
                // First check if we have any issues to show
                if result.issues.is_empty() {
                    continue;
                }

                // Add separator for this tool's output
                output.push_str(&format!("\n🦀 {} results:\n", result.tool_name));

                if let Some(stderr) = &result.stderr {
                    if !stderr.trim().is_empty() {
                        // For clippy, the important info is in stderr
                        // Add some formatting to make it easier to read
                        let formatted_stderr = stderr
                            .lines()
                            .map(|line| {
                                // Add colors to error/warning lines
                                if line.trim().starts_with("error") {
                                    format!("  🔴 {}", line)
                                } else if line.trim().starts_with("warning") {
                                    format!("  🟠 {}", line)
                                } else if line.contains("-->") {
                                    format!("  📄 {}", line)
                                } else if line.contains("|")
                                    && line.trim().starts_with(char::is_numeric)
                                {
                                    format!("     {}", line)
                                } else if line.trim().starts_with("=") {
                                    format!("  💡 {}", line)
                                } else if line.trim().starts_with("help") {
                                    format!("  💡 {}", line)
                                } else {
                                    format!("     {}", line)
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n");

                        output.push_str(&formatted_stderr);
                        output.push('\n');
                    }
                }

                if let Some(stdout) = &result.stdout {
                    if !stdout.trim().is_empty() {
                        // Sometimes important info can be in stdout
                        output.push_str(stdout);
                        output.push('\n');
                    }
                }

                // Also show fixed count for clippy-fix
                if result.tool_name == "clippy-fix" {
                    let fixed_issues: Vec<_> = result
                        .issues
                        .iter()
                        .filter(|i| {
                            i.severity == crate::models::IssueSeverity::Info
                                && i.message.contains("Fixed")
                        })
                        .collect();

                    if !fixed_issues.is_empty() {
                        for issue in fixed_issues {
                            output.push_str(&format!(
                                "\n  {} {}\n",
                                if self.use_emoji { "✨" } else { "" },
                                issue.message
                            ));
                        }
                    }
                }
            }

            // If we've added clippy output, add a separator
            if !output.is_empty() {
                output.push_str("\n──────────────────────────────────────────\n");
            }
        }

        // Collect issues for non-clippy tools or if we want our regular format
        for result in results {
            for issue in &result.issues {
                // Skip info messages about fixed issues (they're handled above)
                if result.tool_name == "clippy-fix"
                    && issue.severity == crate::models::IssueSeverity::Info
                    && issue.message.contains("Fixed")
                {
                    continue;
                }

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

        // If there are no issues, just return the output so far
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
                        "      ".to_string()
                    } else {
                        prev_line = Some(line);
                        format!("L{:<4}", line)
                    }
                } else {
                    "     ".to_string()
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
        } else if error_count == 0 && warning_count == 0 {
            "SUCCESS"
        } else if error_count == 0 {
            "WARNINGS"
        } else {
            "FAILED"
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
