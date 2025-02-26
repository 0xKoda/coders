use anyhow::Result;
use log;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Represents different views/modes in the application
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Welcome,          // Initial welcome screen
    Configuration,    // Setting up API key/model
    ApiKeyInput,      // Input screen for API key
    FileBrowser,      // Browsing files
    FileSelection,    // Selecting multiple files for context
    Editor,           // Viewing/editing code
    PromptInput,      // Writing a prompt
    Results,          // Viewing results with diffs
    Help,             // Help screen
    Credits,          // Viewing OpenRouter credits
}

/// Processing state for API requests
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessingState {
    Idle,
    Processing,
    Done,
    Error(String),
}

/// Side panel selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Left,
    Right,
}

/// Represents the current state of a file diff
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub original: String,
    pub modified: String,
    pub changes: Vec<Change>,
}

/// Represents a single change in a file
#[derive(Debug, Clone)]
pub struct Change {
    pub change_type: ChangeType,
    pub line_number: usize,
    pub content: String,
}

/// Type of change
#[derive(Debug, Clone)]
pub enum ChangeType {
    Insert,
    Delete,
    Modify,
}

/// OpenRouter credits information
#[derive(Debug, Clone)]
pub struct CreditsInfo {
    pub total_credits: f64,
    pub total_usage: f64,
    pub last_updated: SystemTime,
}

impl Default for CreditsInfo {
    fn default() -> Self {
        Self {
            total_credits: 0.0,
            total_usage: 0.0,
            last_updated: SystemTime::now(),
        }
    }
}

/// Main application state
pub struct App {
    pub running: bool,
    pub mode: AppMode,
    pub active_panel: ActivePanel,
    
    // Configuration
    pub api_key: Option<String>,
    pub selected_model: String,
    pub available_models: Vec<String>,
    
    // File browsing
    pub current_dir: PathBuf,
    pub file_list: Vec<PathBuf>,
    pub selected_file_idx: usize,
    
    // Multi-file selection
    pub selected_files: Vec<PathBuf>,
    pub selected_files_content: Vec<(PathBuf, String)>,
    
    // Code editing
    pub current_file: Option<PathBuf>,
    pub current_file_content: String,
    pub scroll_position: usize,
    
    // Prompt
    pub current_prompt: String,
    
    // Results
    pub current_diff: Option<FileDiff>,
    pub current_diff_file: Option<String>,  // Name of the file currently being viewed
    pub multi_file_diffs: Option<Vec<(String, FileDiff)>>,  // All diffs from a multi-file response
    
    // Processing state
    pub processing_state: ProcessingState,
    pub spinner_frame: usize,
    
    // Messages
    pub message: Option<String>,
    pub message_type: MessageType,
    
    // OpenRouter credits
    pub credits_info: Option<CreditsInfo>,
    
    /// Whether to show credits information
    pub show_credits: bool,
}

/// Message type for status updates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Info,
    Error,
    Success,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            mode: AppMode::Welcome,
            active_panel: ActivePanel::Left,
            
            api_key: None,
            selected_model: "anthropic/claude-3.7-sonnet:beta".to_string(),
            available_models: vec![
                "anthropic/claude-3.7-sonnet:beta".to_string(),
                "google/gemini-2.0-flash-001".to_string(),
            ],
            
            current_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            file_list: Vec::new(),
            selected_file_idx: 0,
            
            current_file: None,
            current_file_content: String::new(),
            scroll_position: 0,
            
            selected_files: Vec::new(),
            selected_files_content: Vec::new(),
            
            current_prompt: String::new(),
            
            current_diff: None,
            current_diff_file: None,
            multi_file_diffs: None,
            
            processing_state: ProcessingState::Idle,
            spinner_frame: 0,
            
            message: None, 
            message_type: MessageType::Info,
            
            credits_info: None,
            show_credits: false,
        }
    }
}

impl App {
    /// Create a new application instance
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Handle key events based on current mode
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match self.mode {
            AppMode::Welcome => self.handle_welcome_key(key),
            AppMode::Configuration => self.handle_config_key(key),
            AppMode::ApiKeyInput => self.handle_api_key_input_key(key),
            AppMode::FileBrowser => self.handle_file_browser_key(key),
            AppMode::FileSelection => self.handle_file_selection_key(key),
            AppMode::Editor => self.handle_editor_key(key),
            AppMode::PromptInput => {
                // Use the multi-file context handler if files are selected
                if !self.selected_files.is_empty() {
                    self.handle_prompt_key_with_context(key)
                } else {
                    self.handle_prompt_key(key)
                }
            },
            AppMode::Results => self.handle_results_key(key),
            AppMode::Help => self.handle_help_key(key),
            AppMode::Credits => self.handle_credits_key(key),
        }
    }
    
    fn handle_welcome_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Char('q') => {
                self.running = false;
            }
            KeyCode::Enter => {
                self.mode = AppMode::Configuration;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn handle_config_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};
        
        match key.code {
            KeyCode::Char('q') => {
                // Return to welcome screen if pressed q
                self.mode = AppMode::Welcome;
            }
            KeyCode::Enter => {
                // If API key is configured, proceed to file browser
                if self.api_key.is_some() {
                    // Save the selected model and API key to config
                    if let Some(api_key) = &self.api_key {
                        crate::config::save_api_key(api_key)?;
                        crate::config::save_preferred_model(&self.selected_model)?;
                    }
                    
                    // Refresh file list and switch to file browser
                    self.refresh_file_list()?;
                    self.mode = AppMode::FileBrowser;
                } else {
                    // If no API key, switch to API key input mode
                    self.current_prompt = String::new(); // Use current_prompt for API key input
                    self.mode = AppMode::ApiKeyInput;
                }
            }
            KeyCode::Tab => {
                // Toggle focus between API key and model selection
                // For now, we'll just handle model selection
                // API key input will be handled through a separate mode
            }
            KeyCode::Up => {
                // Navigate model selection upward
                if let Some(idx) = self.available_models.iter().position(|m| m == &self.selected_model) {
                    if idx > 0 {
                        self.selected_model = self.available_models[idx - 1].clone();
                    }
                }
            }
            KeyCode::Down => {
                // Navigate model selection downward
                if let Some(idx) = self.available_models.iter().position(|m| m == &self.selected_model) {
                    if idx < self.available_models.len() - 1 {
                        self.selected_model = self.available_models[idx + 1].clone();
                    }
                }
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+A to add API key - switch to API key input mode
                self.current_prompt = String::new(); // Use current_prompt for API key input
                self.mode = AppMode::ApiKeyInput;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Handle key events for API key input
    fn handle_api_key_input_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Enter => {
                // Save the API key if it's not empty
                if !self.current_prompt.is_empty() {
                    self.api_key = Some(self.current_prompt.clone());
                    self.message = Some("API Key configured successfully".to_string());
                    self.message_type = MessageType::Success;
                    
                    // Save to config
                    crate::config::save_api_key(&self.current_prompt)?;
                }
                
                // Return to configuration screen
                self.mode = AppMode::Configuration;
            }
            KeyCode::Esc => {
                // Cancel and return to configuration screen
                self.mode = AppMode::Configuration;
            }
            KeyCode::Char(c) => {
                // Add character to API key
                self.current_prompt.push(c);
            }
            KeyCode::Backspace => {
                // Remove last character
                self.current_prompt.pop();
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn handle_file_browser_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Char('q') => {
                // Quit the file browser and return to configuration
                self.mode = AppMode::Welcome;
            }
            KeyCode::Char('h') => {
                // Show help screen
                self.mode = AppMode::Help;
            }
            KeyCode::Char('m') => {
                // Enter multi-file selection mode
                self.selected_files.clear();
                self.selected_files_content.clear();
                self.mode = AppMode::FileSelection;
                self.set_info_message("Multi-file selection mode. Use spacebar to select files, Enter when done.");
            }
            KeyCode::Char('s') => {
                // Select the current file for context without opening it
                if self.selected_file_idx < self.file_list.len() {
                    let path = self.file_list[self.selected_file_idx].clone();
                    if path.is_file() {
                        self.toggle_file_selection(path);
                    } else {
                        self.set_info_message("Can only select files, not directories.");
                    }
                }
            }
            KeyCode::Char('p') => {
                // Enter prompt mode if files are selected
                if !self.selected_files.is_empty() {
                    self.mode = AppMode::PromptInput;
                    self.current_prompt = String::new();
                    self.message = Some(format!("Enter your prompt (using {} selected files as context)...", 
                                               self.selected_files.len()));
                } else {
                    self.set_error_message("No files selected. Use 's' to select files first.");
                }
            }
            KeyCode::Char('l') => {
                // List all selected files
                if self.selected_files.is_empty() {
                    self.set_info_message("No files selected. Use 's' to select files.");
                } else {
                    let files_list = self.selected_files.iter()
                        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.set_info_message(&format!("Selected files ({}): {}", self.selected_files.len(), files_list));
                }
            }
            KeyCode::Char('C') if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) => {
                // Clear all selected files
                self.selected_files.clear();
                self.selected_files_content.clear();
                self.set_success_message("Cleared all selected files");
            }
            KeyCode::Char('c') => {
                // View credits information
                if self.api_key.is_some() {
                    self.mode = AppMode::Credits;
                    self.fetch_credits_info();
                } else {
                    self.set_error_message("API key not configured");
                }
            }
            KeyCode::Up => {
                if self.selected_file_idx > 0 {
                    self.selected_file_idx -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_file_idx < self.file_list.len().saturating_sub(1) {
                    self.selected_file_idx += 1;
                }
            }
            KeyCode::Enter => {
                if self.selected_file_idx < self.file_list.len() {
                    let path = self.file_list[self.selected_file_idx].clone();
                    if path.is_dir() {
                        // Change directory
                        self.current_dir = path;
                        self.refresh_file_list()?;
                    } else {
                        // Open file - using cloned path to avoid borrow checker issues
                        self.open_file(&path)?;
                        self.mode = AppMode::Editor;
                    }
                }
            }
            KeyCode::Backspace => {
                // Go up a directory
                if let Some(parent) = self.current_dir.parent() {
                    self.current_dir = parent.to_path_buf();
                    self.refresh_file_list()?;
                    // Log for debugging
                    eprintln!("Navigated to parent directory: {:?}", self.current_dir);
                } else {
                    // Log for debugging
                    eprintln!("No parent directory available");
                }
            }
            // Add an alternative way to go up a directory
            KeyCode::Char('b') => {
                // Go up a directory (alternative to Backspace)
                if let Some(parent) = self.current_dir.parent() {
                    self.current_dir = parent.to_path_buf();
                    self.refresh_file_list()?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn handle_editor_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Char('q') => {
                self.mode = AppMode::FileBrowser;
            }
            KeyCode::Char('p') => {
                self.mode = AppMode::PromptInput;
                if !self.selected_files.is_empty() {
                    self.message = Some(format!("Enter your prompt (using {} selected files as context)...", 
                                               self.selected_files.len()));
                } else {
                    self.message = Some("Enter your prompt...".to_string());
                }
            }
            KeyCode::Char('c') => {
                // Toggle credits display
                self.show_credits = !self.show_credits;
                self.message = Some(format!("Credits display {}", if self.show_credits { "enabled" } else { "disabled" }));
                
                // If enabling credits and we don't have credits info yet, fetch it
                if self.show_credits && self.credits_info.is_none() {
                    self.fetch_credits();
                }
            }
            KeyCode::Char('s') => {
                // Fix borrowing issue by getting a clone of the path before calling toggle_file_selection
                if let Some(file_path) = self.current_file.clone() {
                    self.toggle_file_selection(file_path);
                }
            }
            KeyCode::Char('C') if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) => {
                self.selected_files.clear();
                self.selected_files_content.clear();
                self.set_success_message("Cleared all selected files");
            }
            KeyCode::Char('l') => {
                // List all selected files
                if self.selected_files.is_empty() {
                    self.set_info_message("No files selected. Use 's' to select the current file.");
                } else {
                    let files_list = self.selected_files.iter()
                        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.set_info_message(&format!("Selected files ({}): {}", self.selected_files.len(), files_list));
                }
            }
            KeyCode::Char('r') => {
                self.fetch_credits();
                self.message = Some("Fetching credits information...".to_string());
            }
            KeyCode::Up => {
                if self.scroll_position > 0 {
                    self.scroll_position -= 1;
                }
            }
            KeyCode::Down => {
                // This is simplified, would need to check against actual content length
                self.scroll_position += 1;
            }
            _ => {
                // No action for other keys in editor mode
            }
        }
        
        Ok(())
    }
    
    fn handle_prompt_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                // Determine which mode to return to based on whether we're in multi-file mode
                if !self.selected_files.is_empty() {
                    self.mode = AppMode::FileSelection;
                } else {
                    self.mode = AppMode::Editor;
                }
                // Clear the prompt when exiting
                self.current_prompt = String::new();
            }
            KeyCode::Enter => {
                // Don't process empty prompts
                if self.current_prompt.trim().is_empty() {
                    self.set_error_message("Prompt cannot be empty");
                    return Ok(());
                }
                
                // Set processing state and start API request
                self.processing_state = ProcessingState::Processing;
                
                // Spawn a tokio task to process the prompt asynchronously
                if let Some(api_key) = &self.api_key {
                    // Check if we're in multi-file mode or single file mode
                    if !self.selected_files.is_empty() {
                        // Multi-file mode
                        let prompt = self.current_prompt.clone();
                        let api_key = api_key.clone();
                        let model = self.selected_model.clone();
                        
                        // Create a context string with all selected files
                        let mut context = String::new();
                        for (file_path, content) in &self.selected_files_content {
                            let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();
                            let language = crate::api::get_file_language(file_path);
                            context.push_str(&format!("\n\n--- File: {} ({})\n{}\n", file_name, language, content));
                        }
                        
                        // Use the first file as the primary file for diff generation
                        if let Some((primary_file, primary_content)) = self.selected_files_content.first() {
                            let primary_file = primary_file.clone();
                            let primary_content = primary_content.clone();
                            let language = crate::api::get_file_language(&primary_file);
                            
                            // Get the current Tokio runtime handle
                            match tokio::runtime::Handle::try_current() {
                                Ok(handle) => {
                                    // Spawn a tokio task to process the prompt
                                    handle.spawn(async move {
                                        // Process the prompt
                                        match crate::api::send_code_modification_request_with_context(
                                            &api_key,
                                            &prompt,
                                            &primary_content,
                                            &context,
                                            None, // No AST for now
                                            &model,
                                            language,
                                        ).await {
                                            Ok(response) => {
                                                // Extract code from the response
                                                let extracted_code = crate::api::extract_code_from_response(&response);
                                                
                                                // If no code was extracted, use the full response
                                                let modified_code = if extracted_code.is_empty() {
                                                    response
                                                } else {
                                                    extracted_code
                                                };
                                                
                                                // Generate diff
                                                let diff = crate::files::smart_merge(&primary_content, &modified_code);
                                                
                                                // Send the result back to the main thread
                                                let _ = crate::tui::PROMPT_RESULT_TX.lock().unwrap().send(
                                                    crate::tui::PromptResult::Success(diff)
                                                );
                                            }
                                            Err(e) => {
                                                // Send an error message
                                                let _ = crate::tui::PROMPT_RESULT_TX.lock().unwrap().send(
                                                    crate::tui::PromptResult::Error(e.to_string())
                                                );
                                            }
                                        }
                                    });
                                }
                                Err(_) => {
                                    self.processing_state = ProcessingState::Error("No Tokio runtime available".to_string());
                                }
                            }
                        } else {
                            self.processing_state = ProcessingState::Error("No files selected".to_string());
                        }
                    } else if let Some(file_path) = &self.current_file {
                        // Single file mode
                        let prompt = self.current_prompt.clone();
                        let api_key = api_key.clone();
                        let model = self.selected_model.clone();
                        let code = self.current_file_content.clone();
                        let file_path = file_path.clone();
                        
                        // Get the language from the file extension
                        let language = crate::api::get_file_language(&file_path);
                        
                        // Get the current Tokio runtime handle
                        match tokio::runtime::Handle::try_current() {
                            Ok(handle) => {
                                // Spawn a tokio task to process the prompt
                                handle.spawn(async move {
                                    // Process the prompt
                                    match crate::api::send_code_modification_request(
                                        &api_key,
                                        &prompt,
                                        &code,
                                        None, // No AST for now
                                        &model,
                                        language,
                                    ).await {
                                        Ok(response) => {
                                            // Extract code from the response
                                            let extracted_code = crate::api::extract_code_from_response(&response);
                                            
                                            // If no code was extracted, use the full response
                                            let modified_code = if extracted_code.is_empty() {
                                                response
                                            } else {
                                                extracted_code
                                            };
                                            
                                            // Generate diff
                                            let diff = crate::files::smart_merge(&code, &modified_code);
                                            
                                            // Send the result back to the main thread
                                            let _ = crate::tui::PROMPT_RESULT_TX.lock().unwrap().send(
                                                crate::tui::PromptResult::Success(diff)
                                            );
                                        }
                                        Err(e) => {
                                            // Send an error message
                                            let _ = crate::tui::PROMPT_RESULT_TX.lock().unwrap().send(
                                                crate::tui::PromptResult::Error(e.to_string())
                                            );
                                        }
                                    }
                                });
                            }
                            Err(_) => {
                                self.processing_state = ProcessingState::Error("No Tokio runtime available".to_string());
                            }
                        }
                    } else {
                        self.processing_state = ProcessingState::Error("No file selected".to_string());
                    }
                } else {
                    self.processing_state = ProcessingState::Error("API key not configured".to_string());
                }
            }
            KeyCode::Char(c) => {
                // Add character to prompt
                self.current_prompt.push(c);
            }
            KeyCode::Backspace => {
                // Remove last character
                self.current_prompt.pop();
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Handle the result of a prompt processing
    pub fn handle_prompt_result(&mut self, result: crate::tui::PromptResult) {
        match result {
            crate::tui::PromptResult::Success(diff) => {
                // Update the diff and switch to results mode
                self.current_diff = Some(diff);
                self.processing_state = ProcessingState::Done;
                self.mode = AppMode::Results;
                self.scroll_position = 0;
                self.current_prompt = String::new();
            }
            crate::tui::PromptResult::MultiFileSuccess(diffs) => {
                // Handle multiple file diffs
                if !diffs.is_empty() {
                    // Store all diffs for later application
                    self.multi_file_diffs = Some(diffs.clone());
                    
                    // Set the first diff as the current one to display
                    if let Some((file_path, diff)) = diffs.first() {
                        self.current_diff = Some(diff.clone());
                        self.current_diff_file = Some(file_path.clone());
                        self.processing_state = ProcessingState::Done;
                        self.mode = AppMode::Results;
                        self.scroll_position = 0;
                        self.current_prompt = String::new();
                        
                        // Set a message indicating multiple files were modified
                        if diffs.len() > 1 {
                            self.set_info_message(&format!("Changes for {} files. Reviewing first file.", diffs.len()));
                        }
                    }
                } else {
                    self.set_error_message("No changes were generated for any files.");
                    self.processing_state = ProcessingState::Error("No changes were generated".to_string());
                }
            }
            crate::tui::PromptResult::Error(error) => {
                // Show error message and stay in prompt mode
                self.set_error_message(&format!("API error: {}", error));
                self.processing_state = ProcessingState::Error(error);
            }
            crate::tui::PromptResult::CreditsInfo(credits_info) => {
                // Update credits information
                self.credits_info = Some(credits_info);
                self.processing_state = ProcessingState::Done;
                self.set_success_message("Credits information updated");
            }
        }
    }
    
    /// Update spinner animation frame
    pub fn update_spinner(&mut self) {
        if self.processing_state == ProcessingState::Processing {
            self.spinner_frame = (self.spinner_frame + 1) % 8;
        }
    }
    
    fn handle_results_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Char('y') => {
                // Apply changes
                if let Some(multi_diffs) = &self.multi_file_diffs {
                    // If we have multiple file diffs, apply all of them
                    if multi_diffs.len() > 1 {
                        if let Err(e) = self.apply_multi_file_changes() {
                            self.set_error_message(&format!("Error applying changes: {}", e));
                        }
                    } else {
                        // Single file in multi-file format
                        if let Some(diff) = &self.current_diff {
                            if let Some(file_name) = &self.current_diff_file {
                                // Find the file path that matches the file name
                                for selected_file in &self.selected_files {
                                    if selected_file.file_name() == Some(std::ffi::OsStr::new(file_name)) {
                                        if let Err(e) = std::fs::write(selected_file, &diff.modified) {
                                            self.set_error_message(&format!("Error writing to file: {}", e));
                                        } else {
                                            // Update current file content if this is the current file
                                            if Some(selected_file) == self.current_file.as_ref() {
                                                self.current_file_content = diff.modified.clone();
                                            }
                                            
                                            // Update selected files content
                                            if let Some(idx) = self.selected_files_content.iter().position(|(path, _)| path == selected_file) {
                                                self.selected_files_content[idx].1 = diff.modified.clone();
                                            }
                                            
                                            self.set_success_message(&format!("Changes applied to {}", file_name));
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                } else if let Some(diff) = &self.current_diff {
                    // Regular single file diff
                    if let Some(path) = &self.current_file {
                        if let Err(e) = std::fs::write(path, &diff.modified) {
                            self.set_error_message(&format!("Error writing to file: {}", e));
                        } else {
                            self.current_file_content = diff.modified.clone();
                            self.set_success_message("Changes applied successfully!");
                        }
                    }
                }
                
                // Clear multi-file diffs after applying
                self.multi_file_diffs = None;
                self.current_diff_file = None;
                
                self.mode = AppMode::Editor;
            }
            KeyCode::Char('n') => {
                // Discard changes
                self.set_info_message("Changes discarded.");
                self.multi_file_diffs = None;
                self.current_diff_file = None;
                self.mode = AppMode::Editor;
            }
            KeyCode::Char('j') | KeyCode::Right => {
                // Next file in multi-file diff
                if self.next_diff_file() {
                    // Already handled in the function
                } else {
                    self.set_info_message("No more files to review.");
                }
            }
            KeyCode::Char('k') | KeyCode::Left => {
                // Previous file in multi-file diff
                if self.prev_diff_file() {
                    // Already handled in the function
                } else {
                    self.set_info_message("This is the first file.");
                }
            }
            KeyCode::Tab => {
                // Toggle active panel
                self.active_panel = match self.active_panel {
                    ActivePanel::Left => ActivePanel::Right,
                    ActivePanel::Right => ActivePanel::Left,
                };
            }
            KeyCode::Up => {
                if self.scroll_position > 0 {
                    self.scroll_position -= 1;
                }
            }
            KeyCode::Down => {
                // This is simplified, would need to check against actual content length
                self.scroll_position += 1;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn handle_help_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                // Return to previous mode
                self.mode = AppMode::FileBrowser;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Refresh the file list based on current directory
    pub fn refresh_file_list(&mut self) -> Result<()> {
        self.file_list.clear();
        
        // Add parent directory if not at root
        if self.current_dir.parent().is_some() {
            self.file_list.push(self.current_dir.join(".."));
        }
        
        // Add directories first
        for entry in std::fs::read_dir(&self.current_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.file_list.push(path);
            }
        }
        
        // Then add files
        for entry in std::fs::read_dir(&self.current_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                self.file_list.push(path);
            }
        }
        
        self.selected_file_idx = 0;
        
        Ok(())
    }
    
    /// Open a file and load its content
    pub fn open_file(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.current_file = Some(path.to_path_buf());
        self.current_file_content = content;
        self.scroll_position = 0;
        Ok(())
    }
    
    /// Set an informational message
    pub fn set_info_message(&mut self, msg: &str) {
        self.message = Some(msg.to_string());
        self.message_type = MessageType::Info;
    }
    
    /// Set an error message
    pub fn set_error_message(&mut self, msg: &str) {
        self.message = Some(msg.to_string());
        self.message_type = MessageType::Error;
    }
    
    /// Set a success message
    pub fn set_success_message(&mut self, msg: &str) {
        self.message = Some(msg.to_string());
        self.message_type = MessageType::Success;
    }
    
    /// Handle key events for file selection mode
    fn handle_file_selection_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Char('q') => {
                // Quit the file selection and return to file browser
                self.mode = AppMode::FileBrowser;
                // Clear selected files
                self.selected_files.clear();
                self.selected_files_content.clear();
            }
            KeyCode::Char('h') => {
                // Show help screen
                self.mode = AppMode::Help;
            }
            KeyCode::Up => {
                if self.selected_file_idx > 0 {
                    self.selected_file_idx -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_file_idx < self.file_list.len().saturating_sub(1) {
                    self.selected_file_idx += 1;
                }
            }
            KeyCode::Char(' ') => {
                // Toggle selection of the current file
                if self.selected_file_idx < self.file_list.len() {
                    let path = self.file_list[self.selected_file_idx].clone();
                    if path.is_file() {
                        // Check if the file is already selected
                        if let Some(index) = self.selected_files.iter().position(|p| p == &path) {
                            // Remove from selected files
                            self.selected_files.remove(index);
                            // Also remove from content if it exists
                            if let Some(content_index) = self.selected_files_content.iter().position(|(p, _)| p == &path) {
                                self.selected_files_content.remove(content_index);
                            }
                        } else {
                            // Add to selected files
                            self.selected_files.push(path.clone());
                            // Try to load the content
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                self.selected_files_content.push((path, content));
                            }
                        }
                    }
                }
            }
            KeyCode::Enter => {
                // If files are selected, proceed to prompt input
                if !self.selected_files.is_empty() {
                    // Clear the prompt before entering prompt mode
                    self.current_prompt = String::new();
                    self.mode = AppMode::PromptInput;
                } else {
                    self.set_error_message("No files selected. Use spacebar to select files.");
                }
            }
            KeyCode::Backspace => {
                // Go up a directory
                if let Some(parent) = self.current_dir.parent() {
                    self.current_dir = parent.to_path_buf();
                    self.refresh_file_list()?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Handle key events for the credits screen
    fn handle_credits_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                // Return to previous mode (usually configuration)
                self.mode = AppMode::FileBrowser;
            }
            KeyCode::Char('r') => {
                // Refresh credits information
                self.fetch_credits_info();
            }
            _ => {
                // Ignore other keys
            }
        }
        
        Ok(())
    }
    
    /// Fetch credits information from OpenRouter
    pub fn fetch_credits_info(&mut self) {
        if let Some(api_key) = &self.api_key {
            let api_key = api_key.clone();
            
            // Set processing state
            self.processing_state = ProcessingState::Processing;
            self.message = Some("Fetching credits information...".to_string());
            
            // Get the current Tokio runtime handle
            match tokio::runtime::Handle::try_current() {
                Ok(handle) => {
                    // Spawn a tokio task to fetch credits info
                    let tx = crate::tui::PROMPT_RESULT_TX.lock().unwrap().clone();
                    
                    handle.spawn(async move {
                        match crate::api::fetch_openrouter_credits(&api_key).await {
                            Ok(credits_info) => {
                                // Send the result back to the main thread
                                let _ = tx.send(
                                    crate::tui::PromptResult::CreditsInfo(credits_info)
                                );
                            }
                            Err(e) => {
                                // Send an error message
                                let _ = tx.send(
                                    crate::tui::PromptResult::Error(e.to_string())
                                );
                            }
                        }
                    });
                }
                Err(_) => {
                    self.processing_state = ProcessingState::Error("No Tokio runtime available".to_string());
                }
            }
        } else {
            self.processing_state = ProcessingState::Error("API key not configured".to_string());
            self.message = Some("API key not configured".to_string());
            self.message_type = MessageType::Error;
        }
    }
    
    /// Toggle selection of a file for multi-file context
    pub fn toggle_file_selection(&mut self, file_path: PathBuf) {
        // Check if the file is already selected
        if let Some(index) = self.selected_files.iter().position(|p| p == &file_path) {
            // Remove from selected files
            self.selected_files.remove(index);
            // Also remove from content if it exists
            if let Some(content_index) = self.selected_files_content.iter().position(|(p, _)| p == &file_path) {
                self.selected_files_content.remove(content_index);
            }
            self.set_info_message(&format!("Removed {} from selection", 
                file_path.file_name().unwrap_or_default().to_string_lossy()));
        } else {
            // Add to selected files
            self.selected_files.push(file_path.clone());
            
            // If this is the current file, use the current content in memory
            let content = if Some(&file_path) == self.current_file.as_ref() {
                self.current_file_content.clone()
            } else {
                // Otherwise read from disk
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => content,
                    Err(e) => {
                        self.set_error_message(&format!("Failed to read file: {}", e));
                        // Remove from selected files since we couldn't read it
                        self.selected_files.retain(|f| f != &file_path);
                        return;
                    }
                }
            };
            
            // Add to selected files content
            self.selected_files_content.push((file_path.clone(), content));
            self.set_success_message(&format!("Added {} to selection. Total: {}", 
                file_path.file_name().unwrap_or_default().to_string_lossy(),
                self.selected_files.len()));
        }
    }
    
    /// Fetch credits information
    pub fn fetch_credits(&mut self) {
        if let Some(api_key) = &self.api_key {
            let api_key = api_key.clone();
            self.processing_state = ProcessingState::Processing;
            self.set_info_message("Fetching credits information...");
            
            let tx = crate::tui::PROMPT_RESULT_TX.lock().unwrap().clone();
            
            // Get the current Tokio runtime handle
            match tokio::runtime::Handle::try_current() {
                Ok(handle) => {
                    handle.spawn(async move {
                        match crate::api::fetch_openrouter_credits(&api_key).await {
                            Ok(credits_info) => {
                                let _ = tx.send(crate::tui::PromptResult::CreditsInfo(credits_info));
                            }
                            Err(e) => {
                                let _ = tx.send(crate::tui::PromptResult::Error(format!("Failed to fetch credits: {}", e)));
                            }
                        }
                    });
                }
                Err(_) => {
                    self.processing_state = ProcessingState::Error("No Tokio runtime available".to_string());
                    self.message = Some("No Tokio runtime available".to_string());
                    self.message_type = MessageType::Error;
                }
            }
        } else {
            self.set_error_message("API key not set. Please configure your API key first.");
        }
    }
    
    /// Get combined content from all selected files
    pub fn get_selected_files_content(&self) -> String {
        let mut content = String::new();
        
        for file_path in &self.selected_files {
            if let Ok(file_content) = std::fs::read_to_string(file_path) {
                content.push_str(&format!("File: {}\n{}\n\n", file_path.display(), file_content));
            }
        }
        
        content
    }
    
    /// Handle prompt key with multi-file support
    pub fn handle_prompt_key_with_context(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                // Determine which mode to return to based on whether we're in multi-file mode
                if !self.selected_files.is_empty() {
                    self.mode = AppMode::FileSelection;
                } else {
                    self.mode = AppMode::Editor;
                }
                // Clear the prompt when exiting
                self.current_prompt = String::new();
            }
            KeyCode::Enter => {
                // Don't process empty prompts
                if self.current_prompt.trim().is_empty() {
                    self.set_error_message("Prompt cannot be empty");
                    return Ok(());
                }
                
                // Set processing state
                self.processing_state = ProcessingState::Processing;
                
                if let Some(api_key) = &self.api_key {
                    // Get the primary file (current file or first selected file)
                    let primary_file = if let Some(current_file) = &self.current_file {
                        current_file.clone()
                    } else if let Some((file_path, _)) = self.selected_files_content.first() {
                        file_path.clone()
                    } else {
                        self.processing_state = ProcessingState::Error("No file selected".to_string());
                        return Ok(());
                    };
                    
                    // Get the primary file content
                    let primary_content = if let Some(current_file) = &self.current_file {
                        if primary_file == *current_file {
                            self.current_file_content.clone()
                        } else if let Ok(content) = std::fs::read_to_string(&primary_file) {
                            content
                        } else {
                            self.processing_state = ProcessingState::Error("Failed to read primary file".to_string());
                            return Ok(());
                        }
                    } else if let Some((_, content)) = self.selected_files_content.first() {
                        content.clone()
                    } else {
                        self.processing_state = ProcessingState::Error("No file content available".to_string());
                        return Ok(());
                    };
                    
                    // Create context from all selected files
                    let mut context = String::new();
                    for (file_path, content) in &self.selected_files_content {
                        if file_path != &primary_file {  // Skip the primary file in context
                            let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();
                            let language = crate::api::get_file_language(file_path);
                            context.push_str(&format!("\n\n--- File: {} ({})\n{}\n", file_name, language, content));
                        }
                    }
                    
                    // Clone values for the async task
                    let prompt = self.current_prompt.clone();
                    let api_key = api_key.clone();
                    let model = self.selected_model.clone();
                    let language = crate::api::get_file_language(&primary_file);
                    let primary_file_name = primary_file.file_name().unwrap_or_default().to_string_lossy().to_string();
                    
                    // Clone selected files for multi-file processing
                    let selected_files_content = self.selected_files_content.clone();
                    
                    // Get the current Tokio runtime handle
                    match tokio::runtime::Handle::try_current() {
                        Ok(handle) => {
                            // Spawn a tokio task to process the prompt
                            let tx = crate::tui::PROMPT_RESULT_TX.lock().unwrap().clone();
                            
                            handle.spawn(async move {
                                // Process the prompt with context
                                match crate::api::send_code_modification_request_with_context(
                                    &api_key,
                                    &prompt,
                                    &primary_content,
                                    &context,
                                    None, // No AST for now
                                    &model,
                                    language,
                                ).await {
                                    Ok(response) => {
                                        // Check if the response contains multiple file edits
                                        if response.contains("```") && (response.contains("File:") || response.contains("file:")) {
                                            // Process multi-file response
                                            let file_edits = crate::api::process_multi_file_response(&response);
                                            
                                            if !file_edits.is_empty() {
                                                // Create diffs for each file
                                                let mut diffs = Vec::new();
                                                
                                                // First, handle the primary file if it's in the response
                                                let mut primary_file_handled = false;
                                                
                                                for (file_name, modified_content) in &file_edits {
                                                    // Check if this is the primary file
                                                    if file_name == &primary_file_name {
                                                        let diff = crate::files::smart_merge(&primary_content, modified_content);
                                                        diffs.push((file_name.clone(), diff));
                                                        primary_file_handled = true;
                                                    } else {
                                                        // Find the original content for this file
                                                        for (path, original_content) in &selected_files_content {
                                                            let path_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                                                            if &path_name == file_name {
                                                                let diff = crate::files::smart_merge(original_content, modified_content);
                                                                diffs.push((file_name.clone(), diff));
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                                
                                                // If the primary file wasn't in the response but was modified
                                                if !primary_file_handled {
                                                    // Extract code for the primary file
                                                    let extracted_code = crate::api::extract_code_from_response(&response);
                                                    if !extracted_code.is_empty() {
                                                        let diff = crate::files::smart_merge(&primary_content, &extracted_code);
                                                        diffs.push((primary_file_name, diff));
                                                    }
                                                }
                                                
                                                // Send the multi-file result
                                                let _ = tx.send(crate::tui::PromptResult::MultiFileSuccess(diffs));
                                            } else {
                                                // Fallback to single file if no files were extracted
                                                let extracted_code = crate::api::extract_code_from_response(&response);
                                                let modified_code = if extracted_code.is_empty() { response } else { extracted_code };
                                                let diff = crate::files::smart_merge(&primary_content, &modified_code);
                                                let _ = tx.send(crate::tui::PromptResult::Success(diff));
                                            }
                                        } else {
                                            // Regular single file processing
                                            let extracted_code = crate::api::extract_code_from_response(&response);
                                            let modified_code = if extracted_code.is_empty() { response } else { extracted_code };
                                            let diff = crate::files::smart_merge(&primary_content, &modified_code);
                                            let _ = tx.send(crate::tui::PromptResult::Success(diff));
                                        }
                                    }
                                    Err(e) => {
                                        // Send an error message
                                        let _ = tx.send(crate::tui::PromptResult::Error(e.to_string()));
                                    }
                                }
                            });
                        }
                        Err(_) => {
                            self.processing_state = ProcessingState::Error("No Tokio runtime available".to_string());
                        }
                    }
                } else {
                    self.processing_state = ProcessingState::Error("API key not configured".to_string());
                }
            }
            KeyCode::Char(c) => {
                // Add character to prompt
                self.current_prompt.push(c);
            }
            KeyCode::Backspace => {
                // Remove last character
                self.current_prompt.pop();
            }
            _ => {
                // Ignore other keys
            }
        }
        
        Ok(())
    }
    
    /// Apply changes to all files in a multi-file diff
    pub fn apply_multi_file_changes(&mut self) -> Result<(), anyhow::Error> {
        if let Some(diffs) = &self.multi_file_diffs {
            for (file_path, diff) in diffs {
                // Find the file in the selected files
                for selected_file in &self.selected_files {
                    if selected_file.file_name() == Some(std::ffi::OsStr::new(file_path)) {
                        // Write the modified content to the file
                        std::fs::write(selected_file, &diff.modified)?;
                        
                        // If this is the current file, update the content in memory
                        if Some(selected_file) == self.current_file.as_ref() {
                            self.current_file_content = diff.modified.clone();
                        }
                        
                        // Update the selected files content
                        if let Some(idx) = self.selected_files_content.iter().position(|(path, _)| path == selected_file) {
                            self.selected_files_content[idx].1 = diff.modified.clone();
                        }
                        
                        break;
                    }
                }
            }
            
            self.set_success_message(&format!("Applied changes to {} files", diffs.len()));
            self.multi_file_diffs = None;
        }
        
        Ok(())
    }
    
    /// Move to the next file in a multi-file diff
    pub fn next_diff_file(&mut self) -> bool {
        if let Some(diffs) = &self.multi_file_diffs {
            if let Some(current_file) = &self.current_diff_file {
                // Find the current file index
                if let Some(idx) = diffs.iter().position(|(file, _)| file == current_file) {
                    // If there's a next file, move to it
                    if idx + 1 < diffs.len() {
                        let (next_file, next_diff) = &diffs[idx + 1];
                        self.current_diff = Some(next_diff.clone());
                        self.current_diff_file = Some(next_file.clone());
                        self.scroll_position = 0;
                        self.set_info_message(&format!("Reviewing file {} of {}: {}", 
                                                     idx + 2, diffs.len(), next_file));
                        return true;
                    }
                }
            }
        }
        
        false
    }
    
    /// Move to the previous file in a multi-file diff
    pub fn prev_diff_file(&mut self) -> bool {
        if let Some(diffs) = &self.multi_file_diffs {
            if let Some(current_file) = &self.current_diff_file {
                // Find the current file index
                if let Some(idx) = diffs.iter().position(|(file, _)| file == current_file) {
                    // If there's a previous file, move to it
                    if idx > 0 {
                        let (prev_file, prev_diff) = &diffs[idx - 1];
                        self.current_diff = Some(prev_diff.clone());
                        self.current_diff_file = Some(prev_file.clone());
                        self.scroll_position = 0;
                        self.set_info_message(&format!("Reviewing file {} of {}: {}", 
                                                     idx, diffs.len(), prev_file));
                        return true;
                    }
                }
            }
        }
        
        false
    }
} 