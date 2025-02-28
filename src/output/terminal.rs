use chrono;
use colored::Colorize;
use console::Style;
use std::io::{stdout, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Helper functions for styling
/// Get a style for error messages
pub fn error_style() -> Style {
    Style::new().red().bold()
}

/// Get a style for highlighted text
pub fn highlight_style() -> Style {
    Style::new().magenta().italic()
}

/// Styled section header
pub fn section_header(title: &str) -> String {
    // Create a more interesting header with cyberpunk styling but without box ends
    let prefix = "╸⟪ ";
    let suffix = " ⟫╺";

    format!(
        "\n{}\n",
        format!("{}{}{}", prefix, title, suffix)
            .bright_cyan()
            .on_black()
            .bold()
    )
}

/// Styled divider line
pub fn divider() -> String {
    // Create a more subtle divider that complements the neon aesthetic
    let divider_pattern = "┈".repeat(50);

    // Apply neon styling for a more vibrant but subtle look
    format!("  {}", divider_pattern.bright_cyan().dimmed())
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
    let error_style = error_style();

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

/// Spinner status enum
#[derive(Clone)]
enum SpinnerStatus {
    Active,
    Success(String),
    Warning(String),
    Error(String),
}

/// Status display with a clean, consistent theme
pub struct NeonDisplay {
    spinner_states: Arc<Mutex<Vec<(String, SpinnerStatus)>>>,
    render_thread: Option<thread::JoinHandle<()>>,
    running: Arc<Mutex<bool>>,
    issues_count: usize,
}

impl Default for NeonDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl NeonDisplay {
    /// Create a new status display
    pub fn new() -> Self {
        // Don't clear the terminal, users want to see previous output

        // Clean, elegant header with enhanced neon styling
        let now = chrono::Local::now();
        println!(
            "  {} {} {}",
            "siren".bright_magenta().bold(),
            now.format("%H:%M:%S").to_string().bright_blue().bold(),
            "scan initialized".bright_cyan().bold()
        );

        // Add a subtle separator after the header
        println!("  {}", "┈".repeat(20).bright_cyan().dimmed());

        println!(
            "  {}",
            "Analyzing codebase for quality issues..."
                .bright_white()
                .bold()
        );

        // Add a blank line before spinners start
        println!();

        // Create shared state
        let spinner_states = Arc::new(Mutex::new(Vec::new()));
        let running = Arc::new(Mutex::new(true));

        // Clone for the render thread
        let spinner_states_clone = Arc::clone(&spinner_states);
        let running_clone = Arc::clone(&running);

        // Start the render thread
        let render_thread = thread::spawn(move || {
            // Enhanced spinner frames for a more cyberpunk feel
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut frame_index = 0;

            // Keep track of the number of lines we've printed
            let mut last_render_count = 0;

            while *running_clone.lock().unwrap() {
                // Get the current state
                let states = spinner_states_clone.lock().unwrap().clone();

                // If we have nothing to render, sleep and continue
                if states.is_empty() {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                // Clear previous render by moving cursor up and erasing lines
                for _ in 0..last_render_count {
                    print!("\x1B[1A"); // Move up one line
                    print!("\x1B[2K"); // Clear entire line
                }

                // Count how many lines we'll render this time
                let mut render_count = 0;

                // Render all spinners with enhanced styling
                for (message, status) in &states {
                    match status {
                        SpinnerStatus::Active => {
                            // Active spinner with animation and enhanced styling
                            println!("  {} {}", frames[frame_index].bright_cyan().bold(), message);
                        }
                        SpinnerStatus::Success(details) => {
                            // Success with checkmark and enhanced styling
                            println!(
                                "  {} {} {}",
                                "✓".bright_green().bold(),
                                message,
                                details.bright_green().bold()
                            );
                        }
                        SpinnerStatus::Warning(details) => {
                            // Warning with warning symbol and enhanced styling
                            println!(
                                "  {} {} {}",
                                "⚠".yellow().bold(),
                                message,
                                details.yellow().bold()
                            );
                        }
                        SpinnerStatus::Error(details) => {
                            // Error with x mark and enhanced styling
                            println!(
                                "  {} {} {}",
                                "✗".bright_red().bold(),
                                message,
                                details.bright_red().bold()
                            );
                        }
                    }
                    render_count += 1;
                }

                // Update frame index
                frame_index = (frame_index + 1) % frames.len();

                // Remember how many lines we rendered
                last_render_count = render_count;

                // Flush output
                stdout().flush().unwrap_or(());

                // Sleep briefly
                thread::sleep(Duration::from_millis(80));
            }

            // Final render of all spinners
            let states = spinner_states_clone.lock().unwrap().clone();

            // Clear previous render
            for _ in 0..last_render_count {
                print!("\x1B[1A"); // Move up one line
                print!("\x1B[2K"); // Clear entire line
            }

            // Print all spinners in their final state
            for (message, status) in states {
                match status {
                    SpinnerStatus::Active => {
                        // Show completed for any remaining active spinners with enhanced styling
                        println!(
                            "  {} {} {}",
                            "✓".bright_green().bold(),
                            message,
                            "completed".bright_green().bold()
                        );
                    }
                    SpinnerStatus::Success(details) => {
                        println!(
                            "  {} {} {}",
                            "✓".bright_green().bold(),
                            message,
                            details.bright_green().bold()
                        );
                    }
                    SpinnerStatus::Warning(details) => {
                        println!(
                            "  {} {} {}",
                            "⚠".yellow().bold(),
                            message,
                            details.yellow().bold()
                        );
                    }
                    SpinnerStatus::Error(details) => {
                        println!(
                            "  {} {} {}",
                            "✗".bright_red().bold(),
                            message,
                            details.bright_red().bold()
                        );
                    }
                }
            }
        });

        Self {
            spinner_states,
            render_thread: Some(render_thread),
            running,
            issues_count: 0,
        }
    }

    /// Add a tool status to the display
    pub fn add_tool_status(&mut self, tool_name: &str, language: &str, tool_type: &str) -> usize {
        // Create a cleaner status message with enhanced neon styling
        let status_message = format!(
            "{} {}",
            tool_name.bright_magenta().bold(),
            format!("({} {})", language, tool_type).bright_blue().bold()
        );

        // Add to spinner states
        let mut states = self.spinner_states.lock().unwrap();
        let index = states.len();
        states.push((status_message, SpinnerStatus::Active));

        index
    }

    /// Finish a specific tool with a result message
    pub fn finish_spinner(&mut self, index: usize, message: String) {
        // Update the spinner state
        let mut states = self.spinner_states.lock().unwrap();
        if index < states.len() {
            // Parse the message to determine the status
            let (message_text, status) = if message.contains("no changes needed") {
                // No changes needed - show as info/success with enhanced styling
                let details = "「no changes needed」".to_string();
                (states[index].0.clone(), SpinnerStatus::Success(details))
            } else if message.contains("files formatted") || message.contains("beautified") {
                // Files were formatted - show as success with count and enhanced styling
                let details = if let Some(count) = message
                    .split_whitespace()
                    .find(|s| s.parse::<usize>().is_ok())
                {
                    format!("「{} files formatted」", count)
                } else {
                    "「files formatted」".to_string()
                };
                (states[index].0.clone(), SpinnerStatus::Success(details))
            } else if message.contains("issues found") || message.contains("warnings") {
                // Issues found - show as warning with enhanced styling
                let details = if let Some(count) = message
                    .split_whitespace()
                    .find(|s| s.parse::<usize>().is_ok())
                {
                    format!("「{} issues found」", count)
                } else {
                    "「issues found」".to_string()
                };
                (states[index].0.clone(), SpinnerStatus::Warning(details))
            } else if message.contains("failed") || message.contains("error") {
                // Error occurred - show as error with enhanced styling
                let details = "「execution failed」".to_string();
                (states[index].0.clone(), SpinnerStatus::Error(details))
            } else {
                // Default to success with enhanced styling
                let details = "「completed」".to_string();
                (states[index].0.clone(), SpinnerStatus::Success(details))
            };

            // Update the spinner state
            states[index] = (message_text, status);

            // Add a small delay to make the status change more visible
            drop(states);
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    }

    /// Finish all tools and show the footer
    pub fn finish(&mut self, total_issues: usize) {
        self.issues_count = total_issues;

        // Signal the render thread to stop
        if let Ok(mut running) = self.running.lock() {
            *running = false;
        }

        // Wait for the render thread to finish
        if let Some(handle) = self.render_thread.take() {
            let _ = handle.join();
        }

        // Display elegant footer with enhanced neon styling
        let now = chrono::Local::now();

        // Add a subtle separator before the footer
        println!("\n  {}", "┈".repeat(20).bright_cyan().dimmed());

        println!(
            "  {} {} {}",
            "siren".bright_magenta().bold(),
            now.format("%H:%M:%S").to_string().bright_blue().bold(),
            "scan complete".bright_cyan().bold()
        );

        // We'll skip printing the issue count here since it will be in the summary
    }
}

impl Drop for NeonDisplay {
    fn drop(&mut self) {
        // Signal the render thread to stop
        if let Ok(mut running) = self.running.lock() {
            *running = false;
        }

        // Wait for the render thread to finish
        if let Some(handle) = self.render_thread.take() {
            let _ = handle.join();
        }
    }
}
