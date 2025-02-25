use chrono;
use colored::Colorize;
use console::Style;

/// Terminal UI utilities for Siren
pub struct EnchantedColors;

impl EnchantedColors {
    pub fn primary() -> Style {
        Style::new().magenta().bold()
    }

    pub fn error() -> Style {
        Style::new().red().bold()
    }

    pub fn subtle() -> Style {
        Style::new().dim()
    }

    pub fn highlight() -> Style {
        Style::new().magenta().italic()
    }
}

/// Styled section header
pub fn section_header(title: &str) -> String {
    // Create a more interesting header with cyberpunk styling but without box ends
    let prefix = "╸⟪ ";
    let suffix = " ⟫╺";

    format!(
        "\n{}\n",
        EnchantedColors::primary().apply_to(format!("{}{}{}", prefix, title, suffix))
    )
}

/// Styled divider line
pub fn divider() -> String {
    // Create a more interesting divider with a mix of characters for a cyberpunk feel
    let divider_pattern = "═━═━═━═━═━═━═━═━═━═━═━═━═━═━═━═━═━═━═━═━═━═━═━═━═━═━═━";

    // Apply subtle styling without box ends for a cleaner look
    EnchantedColors::subtle()
        .apply_to(divider_pattern)
        .to_string()
}

/// Tool emoji based on the tool type
pub fn tool_emoji(tool_type: &crate::models::ToolType) -> &'static str {
    use crate::models::ToolType;
    match tool_type {
        ToolType::Linter => "🔍",
        ToolType::Formatter => "💅",
        ToolType::TypeChecker => "🔎",
        ToolType::Fixer => "🧹",
    }
}

/// Language emoji based on the language
pub fn language_emoji(language: &crate::models::Language) -> &'static str {
    use crate::models::Language;
    match language {
        Language::Rust => "🦀",
        Language::Python => "🐍",
        Language::JavaScript => "🌐",
        Language::TypeScript => "📘",
        Language::Html => "🖥️",
        Language::Css => "🎨",
        Language::Go => "🐹",
        Language::Ruby => "💎",
        Language::Markdown => "📝",
        Language::Toml => "📁",
        Language::Json => "📋",
        Language::Yaml => "📄",
        Language::Cpp => "🔵",
        Language::CSharp => "🟢",
        Language::Java => "☕",
        Language::Swift => "🔶",
        Language::C => "🔍",
        Language::Php => "🐘",
        Language::Docker => "🐳",
        Language::Makefile => "🔨",
        Language::Unknown => "📄",
    }
}

/// Display a colorful error panel with a title and message
pub fn error_panel(title: &str, message: &str, details: Option<&str>) {
    let panel_width = 80;
    let separator = "═".repeat(panel_width);
    let error_style = EnchantedColors::error();

    println!("\n{}", error_style.apply_to(format!("╔{}╗", separator)));

    // Title centered
    let title = format!(" {} ", title);
    let padding = (panel_width - title.len()) / 2;
    let title_line =
        " ".repeat(padding) + &title + &" ".repeat(panel_width - padding - title.len());
    println!("{}", error_style.apply_to(format!("║{}║", title_line)));

    println!(
        "{}",
        error_style.apply_to(format!("╠{}╣", "═".repeat(panel_width)))
    );

    // Message with word wrapping
    let words = message.split_whitespace().collect::<Vec<_>>();
    let mut line = String::new();

    for word in words {
        if line.len() + word.len() + 1 > panel_width - 4 {
            let padding = " ".repeat(panel_width - line.len() - 2);
            println!(
                "{}",
                error_style.apply_to(format!("║  {}{}  ║", line, padding))
            );
            line = word.to_string();
        } else {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
    }

    if !line.is_empty() {
        let padding = " ".repeat(panel_width - line.len() - 4);
        println!(
            "{}",
            error_style.apply_to(format!("║  {}{}  ║", line, padding))
        );
    }

    // If there are details, add them
    if let Some(details) = details {
        println!(
            "{}",
            error_style.apply_to(format!("╠{}╣", "═".repeat(panel_width)))
        );

        let words = details.split_whitespace().collect::<Vec<_>>();
        let mut line = String::new();

        for word in words {
            if line.len() + word.len() + 1 > panel_width - 4 {
                let padding = " ".repeat(panel_width - line.len() - 4);
                println!(
                    "{}",
                    error_style.apply_to(format!("║  {}{}  ║", line, padding))
                );
                line = word.to_string();
            } else {
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(word);
            }
        }

        if !line.is_empty() {
            let padding = " ".repeat(panel_width - line.len() - 4);
            println!(
                "{}",
                error_style.apply_to(format!("║  {}{}  ║", line, padding))
            );
        }
    }

    println!("{}", error_style.apply_to(format!("╚{}╝", separator)));
}

/// Status display with a clean, consistent theme
#[derive(Default)]
pub struct NeonDisplay {
    tool_statuses: Vec<String>,
    issues_count: usize,
}

impl NeonDisplay {
    /// Create a new status display
    pub fn new() -> Self {
        // Don't clear the terminal, users want to see previous output

        // Clean, elegant header
        let now = chrono::Local::now();
        println!(
            "{} {} {}",
            "siren".bright_magenta(),
            now.format("%H:%M:%S").to_string().bright_blue(),
            "scan initialized".bright_cyan()
        );
        println!(
            "{}",
            "Analyzing codebase for quality issues...".bright_white()
        );

        Self {
            tool_statuses: Vec::new(),
            issues_count: 0,
        }
    }

    /// Add a tool status to the display
    pub fn add_tool_status(&mut self, tool_name: &str, language: &str, tool_type: &str) -> usize {
        let index = self.tool_statuses.len();

        // Create a cleaner status message with less redundancy
        let message = format!(
            "⚡ {} {}",
            tool_name.bright_magenta(),
            format!("({} {})", language, tool_type).bright_blue()
        );

        // Print the initial status message
        println!("{}", message);

        // Store the message
        self.tool_statuses.push(message);

        index
    }

    /// Finish a specific tool with a result message
    pub fn finish_spinner(&mut self, index: usize, message: String) {
        if index < self.tool_statuses.len() {
            self.tool_statuses[index] = message.clone();
            // Don't print the message again - it will be shown in the summary
        }
    }

    /// Finish all tools and show the footer
    pub fn finish(&mut self, total_issues: usize) {
        self.issues_count = total_issues;

        // Display elegant footer
        let now = chrono::Local::now();
        println!(
            "\n{} {} {}",
            "siren".bright_magenta(),
            now.format("%H:%M:%S").to_string().bright_blue(),
            "scan complete".bright_cyan()
        );

        // We'll skip printing the issue count here since it will be in the summary
    }
}
