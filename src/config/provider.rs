use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Context;
use log::debug;
use crate::errors::ConfigError;
use crate::config::SirenConfig;

/// Trait for providing configuration to the application
pub trait ConfigProvider {
    /// Load configuration from a base directory
    fn load_config(&self, base_dir: &Path) -> Result<SirenConfig, ConfigError>;
}

/// TOML-based configuration provider
#[derive(Debug, Default)]
pub struct TomlConfigProvider {
    /// Global configuration file path
    global_config_path: Option<PathBuf>,
}

impl TomlConfigProvider {
    /// Create a new TOML configuration provider
    pub fn new() -> Self {
        Self {
            global_config_path: None,
        }
    }

    /// Set global configuration path
    pub fn with_global_config(mut self, path: PathBuf) -> Self {
        self.global_config_path = Some(path);
        self
    }

    /// Get path to user global config
    fn get_global_config_path() -> Option<PathBuf> {
        let mut path = dirs::config_dir()?;
        path.push("siren");
        path.push("config.toml");
        Some(path)
    }

    /// Find project-level config by traversing directory tree upwards
    fn find_project_config(&self, base_dir: &Path) -> Option<PathBuf> {
        let mut current_dir = base_dir.to_path_buf();
        
        loop {
            // Check for .siren.toml in current directory
            let config_path = current_dir.join(".siren.toml");
            if config_path.exists() {
                return Some(config_path);
            }
            
            // Also check for siren.toml
            let alt_config_path = current_dir.join("siren.toml");
            if alt_config_path.exists() {
                return Some(alt_config_path);
            }
            
            // Try parent directory, break if no parent
            if !current_dir.pop() {
                break;
            }
        }
        
        None
    }

    /// Read configuration from a file
    fn read_config_file(&self, path: &Path) -> Result<SirenConfig, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::LoadError {
                path: path.to_path_buf(),
                message: "File does not exist".to_string(),
            });
        }
        
        let content = fs::read_to_string(path).map_err(|e| ConfigError::LoadError {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
        
        let config: SirenConfig = toml::from_str(&content).map_err(ConfigError::Toml)?;
        Ok(config)
    }
}

impl ConfigProvider for TomlConfigProvider {
    fn load_config(&self, base_dir: &Path) -> Result<SirenConfig, ConfigError> {
        let mut config = SirenConfig::default();
        
        // Try to load global config first
        let global_config_path = match &self.global_config_path {
            Some(path) => Some(path.clone()),
            None => Self::get_global_config_path(),
        };
        
        if let Some(global_path) = global_config_path {
            if global_path.exists() {
                debug!("Loading global config from {:?}", global_path);
                config = self.read_config_file(&global_path)?;
            }
        }
        
        // Try to load project config to override global config
        if let Some(project_path) = self.find_project_config(base_dir) {
            debug!("Loading project config from {:?}", project_path);
            let project_config = self.read_config_file(&project_path)?;
            
            // Merge project config into global config
            // Override general config
            config.general = project_config.general;
            
            // Override style config
            config.style = project_config.style;
            
            // Merge language configs, project config takes precedence
            for (lang, lang_config) in project_config.languages {
                config.languages.insert(lang, lang_config);
            }
            
            // Merge tool configs, project config takes precedence
            for (tool, tool_config) in project_config.tools {
                config.tools.insert(tool, tool_config);
            }
        }
        
        Ok(config)
    }
} 