//! Output formatting for Siren

pub mod terminal;

use crate::config::OutputConfig;
use crate::models::{IssueSeverity, Language, LintIssue, LintResult, ProjectInfo, ToolType};
use colored::Colorize;
use log::debug;
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use terminal::{language_emoji, tool_emoji};

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

// Helper function to convert absolute paths to relative paths
fn make_relative_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        if let Ok(current_dir) = env::current_dir() {
            if let Ok(relative) = path.strip_prefix(&current_dir) {
                return relative.to_path_buf();
            }
        }
    }
    path.to_path_buf()
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

        // Group results by tool
        for result in results {
            // Get tool info if available
            let tool_info = result.tool.as_ref();
            let tool_type = tool_info.map(|t| t.tool_type);
            let languages = tool_info.map(|t| &t.languages);
            let available = tool_info.map(|t| t.available).unwrap_or(false);
            let version = tool_info.and_then(|t| t.version.as_ref());

            // Group issues by severity
            let mut issues_by_severity: BTreeMap<IssueSeverity, Vec<&LintIssue>> = BTreeMap::new();
            for issue in &result.issues {
                issues_by_severity
                    .entry(issue.severity)
                    .or_default()
                    .push(issue);
            }

            // Skip empty results for linters
            if tool_type
                .as_ref()
                .is_some_and(|tt| matches!(tt, ToolType::Linter))
                && result.issues.is_empty()
            {
                continue;
            }

            // Format tool info
            let mut tool_info_str = String::new();
            if let Some(tt) = tool_type.as_ref() {
                tool_info_str.push_str(&format!(" ({:?})", tt));
            }
            if let Some(languages) = languages {
                let language_str = if languages.len() == 1 {
                    format!("{:?}", languages[0])
                } else {
                    format!("{:?}", languages)
                };
                tool_info_str.push_str(&format!(" for {}", language_str));
            }
            if let Some(version) = version {
                tool_info_str.push_str(&format!(" v{}", version));
            }
            if !available {
                tool_info_str.push_str(" [not available]");
            }

            // Add tool header with emoji, name, and version
            let language_emoji = if let Some(languages) = languages {
                if languages.len() == 1 {
                    self.get_language_emoji(&languages[0])
                } else {
                    "🔧" // Use a generic tool emoji for multi-language tools
                }
            } else {
                "📄" // Default emoji for unknown language
            };
            let tool_emoji = match tool_type.as_ref() {
                Some(tt) => self.get_tool_emoji(tt),
                None => "🔧", // Default emoji for unknown tool type
            };

            // For formatters, if all issues are our special "File formatted" issues, show success
            let tool_status = match tool_type.as_ref() {
                Some(tt) => match tt {
                    ToolType::Formatter
                        if result
                            .issues
                            .iter()
                            .all(|i| i.message == "File formatted successfully") =>
                    {
                        "✨ Success".bright_green()
                    }
                    _ if result.issues.is_empty() => "0 issues".to_string().bright_green(),
                    _ => format!("{} issues", result.issues.len()).bright_yellow(),
                },
                None if result.issues.is_empty() => "0 issues".to_string().bright_green(),
                None => format!("{} issues", result.issues.len()).bright_yellow(),
            };

            // Format the header with tool info - enhanced neon style
            output.push_str("\n\n");

            // Create a more distinct tool header with a subtle background
            output.push_str(&format!(
                "  {}{}{} {} ({})\n",
                language_emoji,
                tool_emoji,
                result.tool_name.bright_magenta().bold(),
                tool_status,
                tool_info_str.bright_blue().dimmed(),
            ));

            // Add a subtle separator under the tool name
            output.push_str(&format!("  {}\n", "┈".repeat(40).bright_cyan().dimmed()));

            // If no issues, add a success message
            if result.issues.is_empty() {
                match tool_type.as_ref() {
                    Some(ToolType::Formatter) => {
                        output.push_str(&format!(
                            "\n  {}\n",
                            "Code beautifully formatted! ✨".bright_green().bold()
                        ));
                    }
                    _ => {
                        output.push_str(&format!(
                            "\n  {}\n",
                            "All checks passed!".bright_green().bold()
                        ));
                    }
                }
                continue;
            }

            // For formatters with only "File formatted" info issues, show a special message
            if tool_type
                .as_ref()
                .is_some_and(|tt| matches!(tt, ToolType::Formatter))
                && result
                    .issues
                    .iter()
                    .all(|i| i.severity == IssueSeverity::Info && i.message == "File formatted")
            {
                let formatted_files = result
                    .issues
                    .iter()
                    .filter_map(|i| i.file.as_ref())
                    .collect::<Vec<_>>();

                if !formatted_files.is_empty() {
                    output.push_str(&format!(
                        "\n  {}\n",
                        "Files formatted:".bright_green().bold()
                    ));
                    for file in formatted_files {
                        let relative_path = make_relative_path(file);
                        output.push_str(&format!(
                            "    {}\n",
                            relative_path.display().to_string().bright_white()
                        ));
                    }
                    continue;
                }
            }

            // Special handling for rustfmt in check mode
            if result.tool_name == "rustfmt" && result.issues.is_empty() {
                // Parse stdout to find files that need formatting
                if let Some(stdout) = &result.stdout {
                    let files_needing_format: Vec<_> = stdout
                        .lines()
                        .filter(|line| !line.trim().is_empty() && !line.contains("Checking"))
                        .collect();

                    if !files_needing_format.is_empty() {
                        if files_needing_format.len() == 1
                            && files_needing_format[0].contains("would be reformatted")
                        {
                            // This is the summary line like "1 file would be reformatted"
                            output.push_str(&format!(
                                "\n  {}\n",
                                files_needing_format[0].bright_yellow().bold()
                            ));
                        } else {
                            output.push_str(&format!(
                                "\n  {}\n",
                                "Files needing format:".bright_yellow().bold()
                            ));
                            for file in files_needing_format {
                                if !file.contains("would be reformatted") {
                                    let path = PathBuf::from(file);
                                    let relative_path = make_relative_path(&path);
                                    output.push_str(&format!(
                                        "    {}\n",
                                        relative_path.display().to_string().bright_white()
                                    ));
                                }
                            }
                        }
                        continue;
                    }
                }
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

            // Add issue count summary with neon styling
            if !errors.is_empty() || !warnings.is_empty() || !info.is_empty() {
                let mut summary = Vec::new();
                if !errors.is_empty() {
                    summary.push(format!(
                        "{} errors",
                        errors.len().to_string().bright_red().bold()
                    ));
                }
                if !warnings.is_empty() {
                    summary.push(format!(
                        "{} style issues",
                        warnings.len().to_string().bright_yellow().bold()
                    ));
                }
                if !info.is_empty() {
                    summary.push(format!(
                        "{} info",
                        info.len().to_string().bright_blue().bold()
                    ));
                }

                output.push_str(&format!("\n  Issues found: {}\n\n", summary.join(", ")));
            }

            // Add the actual issues with enhanced styling
            for issue in &result.issues {
                // Skip our special formatter tracking issues when counting
                if issue.severity == IssueSeverity::Info && issue.message == "File formatted" {
                    continue;
                }

                // Format severity with neon styling
                let severity_str = match issue.severity {
                    IssueSeverity::Error => "error".bright_red().bold(),
                    IssueSeverity::Warning => "style".bright_yellow().bold(),
                    IssueSeverity::Style => "style".bright_magenta().bold(),
                    IssueSeverity::Info => "info".bright_blue().bold(),
                };

                // Format location with neon styling
                let location = if let Some(filepath) = &issue.file {
                    let relative_path = make_relative_path(filepath);
                    if let Some(line) = issue.line {
                        if let Some(column) = issue.column {
                            format!(
                                "{}",
                                format!("{}:{}:{}", relative_path.display(), line, column)
                                    .bright_white()
                            )
                        } else {
                            format!(
                                "{}",
                                format!("{}:{}", relative_path.display(), line).bright_white()
                            )
                        }
                    } else {
                        format!("{}", relative_path.display().to_string().bright_white())
                    }
                } else {
                    "".to_string()
                };

                // Format message
                let message = &issue.message;

                // Format rule if present (using code as rule_id)
                let rule = if let Some(code) = &issue.code {
                    format!(" [{}]", code.bright_cyan())
                } else {
                    "".to_string()
                };

                // Add to output with enhanced styling and indentation
                if !location.is_empty() {
                    output.push_str(&format!(
                        "  {}: {}: {}{}\n",
                        location, severity_str, message, rule
                    ));
                } else {
                    output.push_str(&format!("  {}: {}{}\n", severity_str, message, rule));
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

        // Track tools that were run but had no issues (for linters)
        let mut tools_with_no_issues = 0;
        let mut linters_run = 0;

        // Check if we're only dealing with formatters
        let only_formatters = results.iter().all(|result| {
            result
                .tool
                .as_ref()
                .is_some_and(|t| t.tool_type == ToolType::Formatter)
        });

        // Count of files processed by formatters
        let mut formatted_files = std::collections::HashSet::new();

        // Count issues
        for result in results {
            // Count linters with no issues
            if result
                .tool
                .as_ref()
                .is_some_and(|t| t.tool_type == ToolType::Linter)
            {
                linters_run += 1;
                if result.issues.is_empty() {
                    tools_with_no_issues += 1;
                }
            }

            // Track formatted files if this is a formatter
            if result
                .tool
                .as_ref()
                .is_some_and(|t| t.tool_type == ToolType::Formatter)
            {
                // Check for our special "File formatted" info issues
                let formatted_file_issues = result
                    .issues
                    .iter()
                    .filter(|i| i.severity == IssueSeverity::Info && i.message == "File formatted")
                    .filter_map(|i| i.file.as_ref().cloned())
                    .collect::<Vec<_>>();

                for file in formatted_file_issues {
                    formatted_files.insert(file);
                }

                // Parse rustfmt output with -l flag
                if result.tool_name == "rustfmt" {
                    if let Some(stdout) = &result.stdout {
                        // rustfmt with -l flag outputs one filename per line for files that were formatted
                        for line in stdout.lines() {
                            let line = line.trim();
                            if !line.is_empty() {
                                // With -l flag, rustfmt simply outputs the path of each formatted file
                                // Convert the path to a relative path for better readability
                                let path = PathBuf::from(line);
                                formatted_files.insert(path);
                            }
                        }
                    }
                }

                // Also check stdout as a fallback for other formatters
                if let Some(stdout) = &result.stdout {
                    if stdout.contains("Formatted ") || stdout.contains("Reformatted ") {
                        // Heuristically count files from stdout
                        formatted_files.insert(PathBuf::from("unknown"));
                    }
                }

                // Regular issues (not our special ones)
                for issue in result.issues.iter().filter(|i| {
                    !(i.severity == IssueSeverity::Info && i.message == "File formatted")
                }) {
                    if let Some(filepath) = &issue.file {
                        formatted_files.insert(filepath.clone());
                    }
                }
            }

            for issue in &result.issues {
                // Skip our special formatter tracking issues when counting
                if issue.severity == IssueSeverity::Info && issue.message == "File formatted" {
                    continue;
                }

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

        // Calculate total issues
        let total_issues = error_count + warning_count + style_count + info_count;

        // Add a visual break before the summary section
        output.push_str("\n\n");

        // Add status header with enhanced neon styling
        if only_formatters {
            output.push_str("  ");
            output.push_str(&"✓".bright_green().bold().to_string());
            output.push(' ');
            output.push_str(&"Code style harmonized".bright_green().bold().to_string());
            output.push('\n');
        } else if error_count > 0 {
            output.push_str("  ");
            output.push_str(&"✖".bright_red().bold().to_string());
            output.push(' ');
            output.push_str(
                &"Code quality issues detected"
                    .bright_red()
                    .bold()
                    .to_string(),
            );
            output.push('\n');
        } else if warning_count > 0 || style_count > 0 {
            output.push_str("  ");
            output.push_str(&"⚠".bright_yellow().bold().to_string());
            output.push(' ');
            output.push_str(
                &"Style optimizations available"
                    .bright_yellow()
                    .bold()
                    .to_string(),
            );
            output.push('\n');
        } else {
            output.push_str("  ");
            output.push_str(&"✓".bright_green().bold().to_string());
            output.push(' ');
            output.push_str(
                &"System integrity verified"
                    .bright_green()
                    .bold()
                    .to_string(),
            );
            output.push('\n');
        }

        // Add a stylish separator with neon styling
        output.push_str("  ");
        output.push_str(&"┈".repeat(36).bright_cyan().dimmed().to_string());
        output.push('\n');

        // If we have linters that ran with no issues, show that information
        if linters_run > 0 && tools_with_no_issues > 0 {
            output.push_str(&format!(
                "\n  {} {} {}\n",
                "✓".bright_green().bold(),
                format!("{} of {} linters", tools_with_no_issues, linters_run)
                    .bright_green()
                    .bold(),
                "found no issues".bright_green().bold()
            ));
        }

        // If we have no issues at all, show a success message
        if total_issues == 0 {
            if only_formatters {
                // Special message for formatter-only runs with no issues
                output.push_str("  ");
                output.push_str(&"⚡".bright_green().to_string());
                output.push(' ');

                if formatted_files.is_empty() {
                    output.push_str(
                        &"No files needed formatting"
                            .bright_green()
                            .bold()
                            .to_string(),
                    );
                    output.push('\n');
                } else {
                    // Get relative paths for better readability
                    let relative_formatted_files: Vec<_> = formatted_files
                        .iter()
                        .map(|path| make_relative_path(path))
                        .collect();

                    let count = relative_formatted_files.len();
                    let files_text = if count == 1 { "file" } else { "files" };

                    // Display summary line with enhanced neon styling
                    output.push_str(
                        &format!("{} {} beautified", count, files_text)
                            .bright_green()
                            .bold()
                            .to_string(),
                    );
                    output.push('\n');

                    // Optionally display the list of formatted files with enhanced styling
                    if count <= 5 {
                        // Limit to 5 files to avoid clutter
                        output.push_str("     ");
                        output.push_str(
                            &relative_formatted_files
                                .iter()
                                .map(|path| path.display().to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                                .bright_cyan()
                                .dimmed()
                                .to_string(),
                        );
                        output.push('\n');
                    }
                }
            } else {
                output.push_str("  ");
                output.push_str(&"⚡".bright_green().to_string());
                output.push(' ');
                output.push_str(&"All systems operational".bright_green().bold().to_string());
                output.push('\n');
            }
        } else {
            // Add count summary with enhanced neon styling
            output.push_str("  ");
            output.push_str(&"⚡".bright_cyan().to_string());
            output.push(' ');
            output.push_str(
                &format!("Scan complete: {} issues identified", total_issues)
                    .bright_white()
                    .bold()
                    .to_string(),
            );
            output.push('\n');

            // Create a more detailed breakdown with better spacing and alignment
            let mut details = Vec::new();
            if error_count > 0 {
                details.push(format!(
                    "{} errors",
                    error_count.to_string().bright_red().bold()
                ));
            }
            if warning_count + style_count > 0 {
                details.push(format!(
                    "{} style issues",
                    (warning_count + style_count)
                        .to_string()
                        .bright_yellow()
                        .bold()
                ));
            }
            if info_count > 0 {
                details.push(format!(
                    "{} info notes",
                    info_count.to_string().bright_blue().bold()
                ));
            }

            let details_str = details.join(" • ");
            output.push_str("  ");
            output.push_str(&"▸".bright_magenta().to_string());
            output.push(' ');
            output.push_str(&details_str);
            output.push('\n');

            // Add files affected with enhanced neon styling
            output.push_str("  ");
            output.push_str(&"◈".bright_magenta().to_string());
            output.push(' ');
            output.push_str(&format!(
                "Affected files: {}",
                files_affected.len().to_string().bright_white().bold()
            ));
            output.push('\n');
        }

        // Add breakdown by tool if there are multiple tools with enhanced neon styling
        if tool_counts.len() > 1 && total_issues > 0 {
            output.push('\n');
            output.push_str("  ");
            output.push_str(&"⟁".bright_cyan().to_string());
            output.push(' ');
            output.push_str(&"Issue distribution".bright_cyan().bold().to_string());
            output.push('\n');

            // Sort tools by issue count (descending)
            let mut tools: Vec<_> = tool_counts.iter().collect();
            tools.sort_by(|a, b| b.1.cmp(a.1));

            // Calculate percentages
            let percentage = |count: &i32| {
                if total_issues > 0 {
                    ((*count as f64 / total_issues as f64) * 100.0).round() as i32
                } else {
                    0
                }
            };

            // Format each tool's breakdown with enhanced neon styling
            for (tool_name, count) in tools {
                // Skip tools with no issues
                if *count == 0 {
                    continue;
                }

                let tool_emoji = match tool_name.as_str() {
                    "pylint" => "🐍🔍",
                    "mypy" => "🐍🔎",
                    "ruff" => "🐍🔍",
                    "eslint" => "🌐🔍",
                    "clippy" => "🦀🔍",
                    "rustfmt" => "🦀💅",
                    _ => "🔧",
                };

                let percent = percentage(count);
                let bar_length = (percent as usize * 15) / 100;
                let bar = format!(
                    "{}{}",
                    "█".repeat(bar_length).bright_magenta(),
                    "▒".repeat(15 - bar_length).bright_black()
                );

                output.push_str("  ");
                output.push_str(tool_emoji);
                output.push(' ');
                output.push_str(&format!("{:<8}", tool_name).bright_white().to_string());
                output.push(' ');
                output.push_str(&bar);
                output.push(' ');
                output.push_str(&format!("{:>3}%", percent).bright_cyan().to_string());
                output.push('\n');
            }
        }

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
