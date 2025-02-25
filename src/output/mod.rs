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

    /// Whether to use colors
    use_colors: bool,
}

impl PrettyFormatter {
    /// Create a new PrettyFormatter
    pub fn new() -> Self {
        Self {
            use_emoji: true,
            use_colors: true,
        }
    }

    /// Create a new PrettyFormatter with custom settings
    pub fn with_options(use_emoji: bool, use_colors: bool) -> Self {
        Self {
            use_emoji,
            use_colors,
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

        // Box top
        output.push_str("┌─────────────────────────────────────────────┐\n");

        // Languages
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

            // Format line
            output.push_str(&format!(
                "│ {}{:<10} │ {}{} files{:<4} │",
                lang_emoji,
                format!("{:?}", lang),
                file_emoji,
                file_count,
                ""
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
                output.push_str(&format!(" {}{:<8} │\n", tool_emoji, tool_names.join(", ")));
            } else {
                output.push_str(" │\n");
            }
        }

        // Box bottom
        output.push_str("└─────────────────────────────────────────────┘\n");

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

        // Group results by language
        let mut by_lang: std::collections::HashMap<crate::models::Language, Vec<&LintResult>> =
            std::collections::HashMap::new();

        for result in results {
            if let Some(tool) = &result.tool {
                by_lang.entry(tool.language).or_default().push(result);
            }
        }

        // Format results for each language
        for (lang, lang_results) in by_lang {
            // Language header
            let lang_emoji = if self.use_emoji {
                match lang {
                    crate::models::Language::Rust => "🦀 ",
                    crate::models::Language::Python => "🐍 ",
                    crate::models::Language::JavaScript => "🌐 ",
                    crate::models::Language::TypeScript => "📘 ",
                    _ => "📄 ",
                }
            } else {
                ""
            };

            output.push_str(&format!("\n{}{:?}:\n", lang_emoji, lang));

            // Format results for each tool
            for result in lang_results {
                let tool_name = result
                    .tool
                    .as_ref()
                    .map(|t| t.name.as_str())
                    .unwrap_or("Unknown");

                // Status icon
                let status_icon = if self.use_emoji {
                    if result.success {
                        "✓ "
                    } else if result.issues.is_empty() {
                        "⚠️ "
                    } else {
                        "❌ "
                    }
                } else {
                    if result.success {
                        "[OK] "
                    } else if result.issues.is_empty() {
                        "[WARN] "
                    } else {
                        "[FAIL] "
                    }
                };

                // Tool result summary
                if result.issues.is_empty() {
                    output.push_str(&format!(
                        "  {} {} ({}): All good!\n",
                        status_icon,
                        tool_name,
                        result
                            .tool
                            .as_ref()
                            .map(|t| format!("{:?}", t.tool_type))
                            .unwrap_or_default()
                    ));
                } else {
                    output.push_str(&format!(
                        "  {} {} ({}): {} issues found\n",
                        status_icon,
                        tool_name,
                        result
                            .tool
                            .as_ref()
                            .map(|t| format!("{:?}", t.tool_type))
                            .unwrap_or_default(),
                        result.issues.len()
                    ));

                    // Show issues (limited by config)
                    let issues_to_show =
                        std::cmp::min(result.issues.len(), config.max_issues_per_category);

                    for (_i, issue) in result.issues.iter().take(issues_to_show).enumerate() {
                        // Format issue line
                        let location = if let (Some(file), Some(line)) = (&issue.file, &issue.line)
                        {
                            if config.show_file_paths {
                                format!("{}", file.display())
                            } else {
                                let file_name = file
                                    .file_name()
                                    .map(|f| f.to_string_lossy().to_string())
                                    .unwrap_or_default();

                                if config.show_line_numbers {
                                    format!("{}:{}", file_name, line)
                                } else {
                                    file_name
                                }
                            }
                        } else {
                            String::new()
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

                        output.push_str(&format!(
                            "    └─ {}{}: {}\n",
                            severity_icon,
                            if location.is_empty() {
                                String::new()
                            } else {
                                format!("{} - ", location)
                            },
                            issue.message
                        ));

                        // Show code snippet if available and enabled
                        if config.show_code_snippets && issue.code.is_some() {
                            output.push_str(&format!(
                                "       ```\n       {}\n       ```\n",
                                issue.code.as_ref().unwrap()
                            ));
                        }
                    }

                    // Indicate if there are more issues
                    if result.issues.len() > issues_to_show {
                        output.push_str(&format!(
                            "    └─ [Showing {}/{} issues, use --verbose for all]\n",
                            issues_to_show,
                            result.issues.len()
                        ));
                    }
                }
            }
        }

        output
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

        format!(
            "{} Summary: {} errors, {} warnings",
            status_icon, error_count, warning_count
        )
    }
}

/// JSON formatter for machine-readable output
pub struct JsonFormatter;

impl JsonFormatter {
    pub fn new() -> Self {
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
