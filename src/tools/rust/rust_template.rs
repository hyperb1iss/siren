// TemplateTool implementation
impl LintTool for TemplateTool {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn can_handle(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension() {
            ext == "rs"
        } else {
            false
        }
    }

    fn execute(
        &self,
        files: &[PathBuf],
        config: &ModelsToolConfig,
    ) -> Result<LintResult, ToolError> {
        let start_time = Instant::now();
        let mut issues = Vec::new();
        let mut all_stdout = String::new();
        let mut all_stderr = String::new();

        // Process each file
        for file in files {
            if !self.can_handle(file) {
                continue;
            }

            // Create a command to run the tool on this file
            let mut command = Command::new("your-tool-binary");
            command.arg(file);

            // Add any extra arguments from config
            for arg in &config.extra_args {
                command.arg(arg);
            }

            // Run the command
            let output = command.output().map_err(|e| ToolError::ExecutionFailed {
                name: self.name().to_string(),
                message: format!("Failed to execute {}: {}", self.name(), e),
            })?;

            // Capture stdout and stderr
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            // Append to the combined output
            if !stdout.is_empty() {
                if !all_stdout.is_empty() {
                    all_stdout.push_str("\n\n");
                }
                all_stdout.push_str(&stdout);
            }

            if !stderr.is_empty() {
                if !all_stderr.is_empty() {
                    all_stderr.push_str("\n\n");
                }
                all_stderr.push_str(&stderr);
            }

            // Parse issues from the output
            // TODO: Implement your parsing logic here
            // let file_issues = self.parse_output(&stdout, &stderr);
            // issues.extend(file_issues);
        }

        let execution_time = start_time.elapsed();

        Ok(LintResult {
            tool_name: self.name().to_string(),
            tool: Some(ToolInfo {
                name: self.name().to_string(),
                tool_type: self.tool_type(),
                language: self.language(),
                available: self.is_available(),
                version: self.version(),
                description: self.description().to_string(),
            }),
            success: true, // Tool executed successfully even if issues were found
            issues,
            execution_time,
            stdout: if all_stdout.is_empty() {
                None
            } else {
                Some(all_stdout)
            },
            stderr: if all_stderr.is_empty() {
                None
            } else {
                Some(all_stderr)
            },
        })
    }

    fn tool_type(&self) -> ToolType {
        self.base.tool_type
    }

    fn language(&self) -> Language {
        self.base.language
    }

    fn description(&self) -> &str {
        &self.base.description
    }

    fn is_available(&self) -> bool {
        utils::is_command_available("your-tool-binary")
    }

    fn version(&self) -> Option<String> {
        utils::get_command_version("your-tool-binary", &["--version"])
    }
}
