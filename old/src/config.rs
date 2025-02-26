use anyhow::{Context, Result};
use std::path::PathBuf;

/// Application configuration struct
pub struct Config {
    pub api_key: Option<String>,
    pub preferred_model: String,
    pub enable_ast: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: None,
            preferred_model: "anthropic/claude-3.7-sonnet:beta".to_string(),
            enable_ast: true,
        }
    }
}

impl Config {
    /// Load configuration from disk
    pub fn load() -> Result<Self> {
        let mut config = Self::default();
        
        // Load API key
        if let Some(api_key) = load_api_key() {
            config.api_key = Some(api_key);
        }
        
        // Load preferred model
        if let Some(model) = load_preferred_model() {
            config.preferred_model = model;
        }
        
        // Load AST preference
        if let Some(enable_ast) = load_ast_preference() {
            config.enable_ast = enable_ast;
        }
        
        Ok(config)
    }
    
    /// Save configuration to disk
    pub fn save(&self) -> Result<()> {
        // Save API key if present
        if let Some(api_key) = &self.api_key {
            save_api_key(api_key)?;
        }
        
        // Save preferred model
        save_preferred_model(&self.preferred_model)?;
        
        // Save AST preference
        save_ast_preference(self.enable_ast)?;
        
        Ok(())
    }
}

/// Load API key from configuration
pub fn load_api_key() -> Option<String> {
    let config_dir = dirs::config_dir()?;
    let config_file = config_dir.join("code_ai_openrouter_api_key.txt");
    
    if config_file.exists() {
        if let Ok(api_key) = std::fs::read_to_string(&config_file) {
            let api_key = api_key.trim();
            if !api_key.is_empty() {
                return Some(api_key.to_string());
            }
        }
    }
    
    None
}

/// Save API key to configuration
pub fn save_api_key(api_key: &str) -> Result<()> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?;
    let config_file = config_dir.join("code_ai_openrouter_api_key.txt");
    
    std::fs::create_dir_all(&config_dir)?;
    std::fs::write(config_file, api_key)?;
    
    Ok(())
}

/// Load preferred model from configuration
pub fn load_preferred_model() -> Option<String> {
    let config_dir = dirs::config_dir()?;
    let config_file = config_dir.join("code_ai_preferred_model.txt");
    
    if config_file.exists() {
        if let Ok(model) = std::fs::read_to_string(&config_file) {
            let model = model.trim();
            if !model.is_empty() {
                return Some(model.to_string());
            }
        }
    }
    
    None
}

/// Save preferred model to configuration
pub fn save_preferred_model(model: &str) -> Result<()> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?;
    let config_file = config_dir.join("code_ai_preferred_model.txt");
    
    std::fs::create_dir_all(&config_dir)?;
    std::fs::write(config_file, model)?;
    
    Ok(())
}

/// Load AST preference from configuration
pub fn load_ast_preference() -> Option<bool> {
    let config_dir = dirs::config_dir()?;
    let config_file = config_dir.join("code_ai_enable_ast.txt");
    
    if config_file.exists() {
        if let Ok(enable_ast) = std::fs::read_to_string(&config_file) {
            let enable_ast = enable_ast.trim();
            if !enable_ast.is_empty() {
                return Some(enable_ast == "true");
            }
        }
    }
    
    None
}

/// Save AST preference to configuration
pub fn save_ast_preference(enable_ast: bool) -> Result<()> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?;
    let config_file = config_dir.join("code_ai_enable_ast.txt");
    
    std::fs::create_dir_all(&config_dir)?;
    std::fs::write(config_file, if enable_ast { "true" } else { "false" })?;
    
    Ok(())
}

/// Get the configuration directory path
pub fn config_dir() -> Result<PathBuf> {
    dirs::config_dir()
        .context("Failed to get config directory")
}

/// Reset all configuration
pub fn reset_config() -> Result<()> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?;
    
    let files = [
        "code_ai_openrouter_api_key.txt",
        "code_ai_preferred_model.txt",
        "code_ai_enable_ast.txt",
    ];
    
    for file in &files {
        let file_path = config_dir.join(file);
        if file_path.exists() {
            std::fs::remove_file(&file_path)?;
        }
    }
    
    Ok(())
} 