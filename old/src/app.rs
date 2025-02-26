use anyhow::Result;
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
    
    // Processing state
    pub processing_state: ProcessingState,
    pub spinner_frame: usize,
    
    // Messages
    pub message: Option<String>,
    pub message_type: MessageType,
    
    // OpenRouter credits
    pub credits_info: CreditsInfo,
    
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
            
            processing_state: ProcessingState::Idle,
            spinner_frame: 0,
            
            message: None, 
            message_type: MessageType::Info,
            
            credits_info: CreditsInfo::default(),
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
            AppMode::PromptInput => self.handle_prompt_key(key),
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
                // Clear the prompt before entering prompt mode
                self.current_prompt = String::new();
                self.mode = AppMode::PromptInput;
            }
            KeyCode::Char('c') => {
                // Toggle credits display
                self.toggle_credits_display();
            }
            KeyCode::Char('s') => {
                // Toggle file selection for multi-file context
                self.toggle_file_selection();
            }
            KeyCode::Char('C') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                    // Clear selected files
                    self.clear_selected_files();
                }
            }
            KeyCode::Char('r') => {
                // Refresh credits information
                self.fetch_credits();
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
            crate::tui::PromptResult::Error(error) => {
                // Show error message and stay in prompt mode
                self.set_error_message(&format!("API error: {}", error));
                self.processing_state = ProcessingState::Error(error);
            }
            crate::tui::PromptResult::CreditsInfo(credits_info) => {
                // Update credits information
                self.credits_info = credits_info;
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
                if let Some(diff) = &self.current_diff {
                    if let Some(path) = &self.current_file {
                        std::fs::write(path, &diff.modified)?;
                        self.current_file_content = diff.modified.clone();
                        self.set_success_message("Changes applied successfully!");
                    }
                }
                self.mode = AppMode::Editor;
            }
            KeyCode::Char('n') => {
                // Discard changes
                self.set_info_message("Changes discarded.");
                self.mode = AppMode::Editor;
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
                self.mode = AppMode::Configuration;
            }
            KeyCode::Char('r') => {
                // Refresh credits information
                self.fetch_credits_info();
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Fetch credits information from OpenRouter
    pub fn fetch_credits_info(&mut self) {
        if let Some(api_key) = &self.api_key {
            let api_key = api_key.clone();
            
            // Set processing state
            self.processing_state = ProcessingState::Processing;
            
            // Get the current Tokio runtime handle
            match tokio::runtime::Handle::try_current() {
                Ok(handle) => {
                    // Spawn a tokio task to fetch credits info
                    handle.spawn(async move {
                        match crate::api::fetch_openrouter_credits(&api_key).await {
                            Ok(credits_info) => {
                                // Send the result back to the main thread
                                let _ = crate::tui::PROMPT_RESULT_TX.lock().unwrap().send(
                                    crate::tui::PromptResult::CreditsInfo(credits_info)
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
            self.processing_state = ProcessingState::Error("API key not configured".to_string());
        }
    }
    
    /// Toggle selection of a file for multi-file context
    pub fn toggle_file_selection(&mut self) {
        if let Some(current_file) = &self.current_file {
            if self.selected_files.contains(current_file) {
                self.selected_files.retain(|f| f != current_file);
                self.set_info_message(&format!("Removed {} from selection", current_file.display()));
            } else {
                self.selected_files.push(current_file.clone());
                self.set_info_message(&format!("Added file to selection. Total: {}", self.selected_files.len()));
            }
        }
    }
    
    /// Clear all selected files
    pub fn clear_selected_files(&mut self) {
        self.selected_files.clear();
        self.set_info_message("Cleared file selection");
    }
    
    /// Toggle credits display
    pub fn toggle_credits_display(&mut self) {
        self.show_credits = !self.show_credits;
    }
    
    /// Fetch credits information
    pub fn fetch_credits(&mut self) {
        if let Some(api_key) = &self.api_key {
            let api_key = api_key.clone();
            self.processing_state = ProcessingState::Processing;
            self.set_info_message("Fetching credits information...");
            
            let tx = crate::tui::PROMPT_RESULT_TX.lock().unwrap().clone();
            
            tokio::spawn(async move {
                match crate::api::fetch_openrouter_credits(&api_key).await {
                    Ok(credits_info) => {
                        let _ = tx.send(crate::tui::PromptResult::CreditsInfo(credits_info));
                    }
                    Err(e) => {
                        let _ = tx.send(crate::tui::PromptResult::Error(format!("Failed to fetch credits: {}", e)));
                    }
                }
            });
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
        match key.code {
            // ... existing key handling code ...
            
            crossterm::event::KeyCode::Enter => {
                let prompt = self.current_prompt.clone();
                if prompt.is_empty() {
                    self.set_error_message("Prompt cannot be empty");
                    return Ok(());
                }
                
                if let Some(current_file) = &self.current_file {
                    if let Ok(current_file_content) = std::fs::read_to_string(current_file) {
                        let api_key = self.api_key.clone().unwrap_or_default();
                        let current_file = current_file.clone();
                        let current_prompt = self.current_prompt.clone();
                        let selected_model = self.selected_model.clone();
                        
                        // Get additional context from selected files
                        let additional_context = self.get_selected_files_content();
                        
                        self.processing_state = ProcessingState::Processing;
                        self.spinner_frame = 0;
                        
                        let tx = crate::tui::PROMPT_RESULT_TX.lock().unwrap().clone();
                        
                        tokio::spawn(async move {
                            let language = crate::api::get_file_language(&current_file);
                            
                            // Generate AST if needed
                            let ast = if language == "javascript" || language == "typescript" || language == "python" || language == "rust" {
                                match crate::api::generate_ast(&api_key, &current_file_content, &language).await {
                                    Ok(ast) => Some(ast),
                                    Err(e) => {
                                        log::warn!("Failed to generate AST: {}", e);
                                        None
                                    }
                                }
                            } else {
                                None
                            };
                            
                            // Send request with context
                            let result = if additional_context.is_empty() {
                                // Use regular request if no additional files selected
                                crate::api::send_code_modification_request(
                                    &api_key,
                                    &current_prompt,
                                    &current_file_content,
                                    ast.as_deref(),
                                    &selected_model,
                                    &language,
                                ).await
                            } else {
                                // Use multi-file context request
                                crate::api::send_code_modification_request_with_context(
                                    &api_key,
                                    &current_prompt,
                                    &current_file_content,
                                    &additional_context,
                                    ast.as_deref(),
                                    &selected_model,
                                    &language,
                                ).await
                            };
                            
                            match result {
                                Ok(response) => {
                                    let code = crate::api::extract_code_from_response(&response);
                                    
                                    // smart_merge returns FileDiff directly, not a Result
                                    let diff = crate::files::smart_merge(&current_file_content, &code);
                                    let _ = tx.send(crate::tui::PromptResult::Success(diff));
                                }
                                Err(e) => {
                                    let _ = tx.send(crate::tui::PromptResult::Error(format!("API request failed: {}", e)));
                                }
                            }
                        });
                    } else {
                        self.set_error_message("Failed to read current file");
                    }
                } else {
                    self.set_error_message("No file selected");
                }
                
                self.mode = AppMode::Editor;
            }
            
            // ... existing key handling code ...
        }
        
        Ok(())
    }
} 