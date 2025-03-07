use anyhow::Result;
use log::{debug, info, error};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, Duration, Instant};

/// Represents different views/modes in the application
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Welcome,          // Initial welcome screen
    Configuration,    // Setting up API key/model
    ApiKeyInput,      // Input screen for API key
    CustomModelInput, // Input screen for custom model
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
    pub explanation_text: Option<String>,
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
    pub custom_model: String,
    
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
    pub explanation_text: Option<String>,   // Store the explanatory text from the LLM for the entire response
    
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
    
    /// Message timeout (for auto-clearing messages)
    pub message_time: Option<Instant>,
    pub message_timeout: Duration,
    
    /// Whether to show explanation text in results view
    pub show_explanation: bool,
    
    /// Token count for context window
    pub token_count: usize,
    
    /// Context window size
    pub context_window_size: usize,
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
                "google/gemini-2.0-flash-001".to_string(),
                "anthropic/claude-3.7-sonnet".to_string(),
                "custom".to_string(),  // Option for custom model input
            ],
            custom_model: String::new(),
            
            current_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            file_list: Vec::new(),
            selected_file_idx: 0,
            
            selected_files: Vec::new(),
            selected_files_content: Vec::new(),
            
            current_file: None,
            current_file_content: String::new(),
            scroll_position: 0,
            
            current_prompt: String::new(),
            
            current_diff: None,
            current_diff_file: None,
            multi_file_diffs: None,
            explanation_text: None,
            
            processing_state: ProcessingState::Idle,
            spinner_frame: 0,
            
            message: None,
            message_type: MessageType::Info,
            
            credits_info: None,
            
            show_credits: false,
            
            message_time: None,
            message_timeout: Duration::from_secs(3),
            
            show_explanation: false,
            
            token_count: 0,
            
            context_window_size: 0,
        }
    }
}

impl App {
    /// Create a new application instance
    pub fn new() -> Self {
        let mut app = Self::default();
        app.update_context_window_size();
        app.update_token_count();
        app
    }
    
    /// Handle key events based on current mode
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match self.mode {
            AppMode::Welcome => self.handle_welcome_key(key),
            AppMode::Configuration => self.handle_config_key(key),
            AppMode::ApiKeyInput => self.handle_api_key_input_key(key),
            AppMode::CustomModelInput => self.handle_custom_model_input_key(key),
            AppMode::FileBrowser => self.handle_file_browser_key(key),
            AppMode::FileSelection => self.handle_file_selection_key(key),
            AppMode::Editor => self.handle_editor_key(key),
            AppMode::PromptInput => {
                // Check if we have selected files for context
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
                self.mode = AppMode::Welcome;
            },
            KeyCode::Up => {
                // Get the index of the currently selected model
                let current_idx = self.available_models.iter().position(|m| m == &self.selected_model)
                    .unwrap_or(0);
                
                // Move up in the list (with wraparound)
                if current_idx > 0 {
                    self.selected_model = self.available_models[current_idx - 1].clone();
                } else {
                    self.selected_model = self.available_models[self.available_models.len() - 1].clone();
                }
            },
            KeyCode::Down => {
                // Get the index of the currently selected model
                let current_idx = self.available_models.iter().position(|m| m == &self.selected_model)
                    .unwrap_or(0);
                
                // Move down in the list (with wraparound)
                if current_idx < self.available_models.len() - 1 {
                    self.selected_model = self.available_models[current_idx + 1].clone();
                } else {
                    self.selected_model = self.available_models[0].clone();
                }
            },
            KeyCode::Enter => {
                // If "custom" is selected, go to custom model input
                if self.selected_model == "custom" {
                    self.mode = AppMode::CustomModelInput;
                    self.current_prompt = self.custom_model.clone();
                    return Ok(());
                }
                
                // If API key is set, save the selected model and proceed
                if self.api_key.is_some() {
                    // Save the selected model to config
                    if let Some(config_dir) = dirs::config_dir() {
                        let config_file = config_dir.join("code_ai_preferred_model.txt");
                        std::fs::write(config_file, &self.selected_model)?;
                        self.set_success_message(&format!("Model set to {}", self.selected_model));
                    }
                    
                    // Proceed to file browser
                    self.mode = AppMode::FileBrowser;
                    self.refresh_file_list()?;
                } else {
                    // If no API key, prompt for it
                    self.mode = AppMode::ApiKeyInput;
                    self.current_prompt = String::new();
                }
            },
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+A to set API key
                self.mode = AppMode::ApiKeyInput;
                self.current_prompt = self.api_key.clone().unwrap_or_default();
            },
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
                if let Some(_api_key) = &self.api_key {
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
        
        // If we're already processing, ignore most key presses
        if self.processing_state == ProcessingState::Processing {
            // Only allow Escape to cancel the processing
            if key.code == KeyCode::Esc {
                self.processing_state = ProcessingState::Idle;
                self.set_info_message("Request cancelled");
            }
            return Ok(());
        }
        
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
                // Update token count
                self.update_token_count();
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
                self.current_prompt.push(c);
                // Update token count immediately
                self.update_token_count();
            }
            KeyCode::Backspace => {
                self.current_prompt.pop();
                // Update token count immediately
                self.update_token_count();
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Handle the result of a prompt operation
    pub fn handle_prompt_result(&mut self, result: crate::tui::PromptResult) {
        match result {
            crate::tui::PromptResult::Success(diff) => {
                // Single file response
                self.current_diff = Some(diff.clone());
                
                // Extract the file name from the current file
                if let Some(ref file_path) = self.current_file {
                    if let Some(file_name) = file_path.file_name() {
                        self.current_diff_file = Some(file_name.to_string_lossy().to_string());
                    }
                }
                
                // Store explanation text separately for easy access
                if let Some(ref explanation) = diff.explanation_text {
                    self.explanation_text = Some(explanation.clone());
                }
                
                // Switch to results mode
                self.mode = AppMode::Results;
                self.processing_state = ProcessingState::Done;
                
                // Reset scroll position
                self.scroll_position = 0;
                
                // Log the result
                info!("Received single file diff with {} changes", diff.changes.len());
                if diff.explanation_text.is_some() {
                    info!("Explanation text is available");
                }
            },
            crate::tui::PromptResult::MultiFileSuccess(diffs) => {
                // Multi-file response
                if !diffs.is_empty() {
                    // Store all diffs
                    self.multi_file_diffs = Some(diffs.clone());
                    
                    // Set the current diff to the first file
                    let (file_name, diff) = &diffs[0];
                    self.current_diff_file = Some(file_name.clone());
                    self.current_diff = Some(diff.clone());
                    
                    // Store explanation text separately for easy access
                    if let Some(ref explanation) = diff.explanation_text {
                        self.explanation_text = Some(explanation.clone());
                    }
                    
                    // Switch to results mode
                    self.mode = AppMode::Results;
                    self.processing_state = ProcessingState::Done;
                    
                    // Reset scroll position
                    self.scroll_position = 0;
                    
                    // Log the result
                    info!("Received multi-file diff with {} files", diffs.len());
                    if diff.explanation_text.is_some() {
                        info!("Explanation text is available");
                    }
                } else {
                    // No diffs received
                    self.set_error_message("No changes were generated");
                    self.processing_state = ProcessingState::Error("No changes were generated".to_string());
                }
            },
            crate::tui::PromptResult::Error(err) => {
                // Error response
                self.set_error_message(&format!("Error: {}", err));
                self.processing_state = ProcessingState::Error(err);
            },
            crate::tui::PromptResult::CreditsInfo(credits_info) => {
                // Credits info response
                self.credits_info = Some(credits_info);
                self.show_credits = true;
                self.mode = AppMode::Credits;
                self.processing_state = ProcessingState::Done;
            },
        }
        
        // Log the state of the diff viewer for debugging
        self.log_diff_viewer_state();
    }
    
    /// Update spinner animation frame and check message timeout
    pub fn update(&mut self) {
        // Update spinner animation
        if self.processing_state == ProcessingState::Processing {
            self.spinner_frame = (self.spinner_frame + 1) % 8;
        }
        
        // Check if message should be cleared
        if let Some(time) = self.message_time {
            if time.elapsed() >= self.message_timeout && self.message_type == MessageType::Success {
                self.message = None;
                self.message_time = None;
            }
        }
    }
    
    fn handle_results_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            crossterm::event::KeyCode::Char('q') => {
                self.mode = AppMode::FileBrowser;
                Ok(())
            },
            crossterm::event::KeyCode::Char('y') => {
                // Apply changes
                info!("User pressed 'y' to apply changes");
                
                if let Some(ref multi_diffs) = self.multi_file_diffs {
                    // We have multi-file diffs
                    info!("Found multi-file diffs with {} files", multi_diffs.len());
                    
                    for (filename, _) in multi_diffs {
                        info!("Multi-file diff includes: {}", filename);
                    }
                    
                    match self.apply_multi_file_changes() {
                        Ok(_) => {
                            info!("Successfully applied multi-file changes");
                            self.set_success_message("Successfully applied all changes");
                            // Return to file browser after applying changes
                            self.mode = AppMode::FileBrowser;
                        }
                        Err(e) => {
                            error!("Failed to apply multi-file changes: {}", e);
                            self.set_error_message(&format!("Error applying changes: {}", e));
                        }
                    }
                } else if let Some(ref diff) = self.current_diff {
                    // We have a single file diff
                    if let Some(ref file_path) = self.current_file {
                        info!("Applying changes to single file: {:?}", file_path);
                        let modified_content = diff.modified.clone();
                        match std::fs::write(file_path, &modified_content) {
                            Ok(_) => {
                                info!("Successfully applied changes to {:?}", file_path);
                                self.set_success_message(&format!("Applied changes to {}", 
                                    file_path.file_name().unwrap_or_default().to_string_lossy()));
                                
                                // Update current file content
                                self.current_file_content = modified_content;
                                
                                // Return to file browser after applying changes
                                self.mode = AppMode::FileBrowser;
                            },
                            Err(e) => {
                                error!("Failed to apply changes to {:?}: {}", file_path, e);
                                self.set_error_message(&format!("Failed to apply changes: {}", e));
                            }
                        }
                    } else {
                        error!("No current file selected to apply diff to");
                        self.set_error_message("No file selected to apply changes to");
                    }
                } else {
                    info!("No diffs to apply");
                    self.set_info_message("No changes to apply");
                }
                Ok(())
            },
            crossterm::event::KeyCode::Char('n') => {
                // Reject changes
                self.mode = AppMode::FileBrowser;
                self.set_info_message("Changes rejected");
                Ok(())
            },
            crossterm::event::KeyCode::Char('e') => {
                // Toggle explanation view
                self.toggle_explanation_view();
                Ok(())
            },
            crossterm::event::KeyCode::Tab => {
                // Switch active panel
                self.active_panel = match self.active_panel {
                    ActivePanel::Left => ActivePanel::Right,
                    ActivePanel::Right => ActivePanel::Left,
                };
                Ok(())
            },
            crossterm::event::KeyCode::Right => {
                // Show next file diff
                if self.next_diff_file() {
                    self.set_info_message("Showing next file diff");
                }
                Ok(())
            },
            crossterm::event::KeyCode::Left => {
                // Show previous file diff
                if self.prev_diff_file() {
                    self.set_info_message("Showing previous file diff");
                }
                Ok(())
            },
            crossterm::event::KeyCode::Up => {
                // Scroll up based on active panel
                match self.active_panel {
                    ActivePanel::Left => {
                        if self.scroll_position > 0 {
                            self.scroll_position -= 1;
                        }
                    },
                    ActivePanel::Right => {
                        if self.scroll_position > 0 {
                            self.scroll_position -= 1;
                        }
                    },
                }
                Ok(())
            },
            crossterm::event::KeyCode::Down => {
                // Scroll down based on active panel
                match self.active_panel {
                    ActivePanel::Left => {
                        // Limit scrolling based on content length
                        if let Some(ref diff) = self.current_diff {
                            let line_count = diff.original.lines().count();
                            if self.scroll_position < line_count.saturating_sub(10) {
                                self.scroll_position += 1;
                            }
                        }
                    },
                    ActivePanel::Right => {
                        // Limit scrolling based on content length
                        if let Some(ref diff) = self.current_diff {
                            let line_count = diff.modified.lines().count();
                            if self.scroll_position < line_count.saturating_sub(10) {
                                self.scroll_position += 1;
                            }
                        }
                    },
                }
                Ok(())
            },
            crossterm::event::KeyCode::Home => {
                // Scroll to top
                self.scroll_position = 0;
                Ok(())
            },
            crossterm::event::KeyCode::End => {
                // Scroll to bottom
                if let Some(ref diff) = self.current_diff {
                    let line_count = match self.active_panel {
                        ActivePanel::Left => diff.original.lines().count(),
                        ActivePanel::Right => diff.modified.lines().count(),
                    };
                    self.scroll_position = line_count.saturating_sub(10);
                }
                Ok(())
            },
            crossterm::event::KeyCode::PageUp => {
                // Scroll up by 10 lines
                if self.scroll_position >= 10 {
                    self.scroll_position -= 10;
                } else {
                    self.scroll_position = 0;
                }
                Ok(())
            },
            crossterm::event::KeyCode::PageDown => {
                // Scroll down by 10 lines
                if let Some(ref diff) = self.current_diff {
                    let line_count = match self.active_panel {
                        ActivePanel::Left => diff.original.lines().count(),
                        ActivePanel::Right => diff.modified.lines().count(),
                    };
                    if self.scroll_position + 10 < line_count.saturating_sub(10) {
                        self.scroll_position += 10;
                    } else {
                        self.scroll_position = line_count.saturating_sub(10);
                    }
                }
                Ok(())
            },
            _ => Ok(()),
        }
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
        self.message_time = Some(Instant::now());
    }
    
    /// Set an error message
    pub fn set_error_message(&mut self, msg: &str) {
        self.message = Some(msg.to_string());
        self.message_type = MessageType::Error;
        self.message_time = Some(Instant::now());
    }
    
    /// Set a success message
    pub fn set_success_message(&mut self, msg: &str) {
        self.message = Some(msg.to_string());
        self.message_type = MessageType::Success;
        self.message_time = Some(Instant::now());
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
    
    /// Toggle file selection for multi-file context
    pub fn toggle_file_selection(&mut self, file_path: PathBuf) {
        // Check if the file is already selected
        if let Some(idx) = self.selected_files.iter().position(|p| p == &file_path) {
            // Remove the file from selected_files
            self.selected_files.remove(idx);
            
            // Remove the file content
            self.selected_files_content.retain(|(path, _)| path != &file_path);
            
            // Set info message
            if let Some(file_name) = file_path.file_name() {
                self.set_info_message(&format!("Removed '{}' from selected files", file_name.to_string_lossy()));
            }
        } else {
            // Add the file to selected_files
            self.selected_files.push(file_path.clone());
            
            // Add the file content
            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    self.selected_files_content.push((file_path.clone(), content));
                    
                    // Set success message
                    if let Some(file_name) = file_path.file_name() {
                        self.set_success_message(&format!("Added '{}' to selected files", file_name.to_string_lossy()));
                    }
                }
                Err(e) => {
                    // Set error message
                    self.set_error_message(&format!("Failed to read file: {}", e));
                    
                    // Remove the file from selected_files
                    if let Some(idx) = self.selected_files.iter().position(|p| p == &file_path) {
                        self.selected_files.remove(idx);
                    }
                }
            }
        }
        
        // Update token count after modifying the selection
        self.update_token_count();
        debug!("Updated token count after toggle_file_selection: {}", self.token_count);
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
        
        // If we're already processing, ignore most key presses
        if self.processing_state == ProcessingState::Processing {
            // Only allow Escape to cancel the processing
            if key.code == KeyCode::Esc {
                self.processing_state = ProcessingState::Idle;
                self.set_info_message("Request cancelled");
            }
            return Ok(());
        }
        
        match key.code {
            KeyCode::Esc => {
                // Return to file selection mode
                self.mode = AppMode::FileSelection;
                // Clear the prompt
                self.current_prompt = String::new();
                // Update token count
                self.update_token_count();
            }
            KeyCode::Enter => {
                // Submit prompt with context
                if !self.current_prompt.is_empty() {
                    // Set processing state
                    self.processing_state = ProcessingState::Processing;
                    
                    // Get the API key
                    let api_key = match &self.api_key {
                        Some(key) => key.clone(),
                        None => {
                            self.set_error_message("API key not set");
                            self.processing_state = ProcessingState::Error("API key not set".to_string());
                            return Ok(());
                        }
                    };
                    
                    // Get the selected model
                    let model = self.selected_model.clone();
                    
                    // Get the current file content
                    let current_file_content = self.current_file_content.clone();
                    
                    // Get the current file path
                    let current_file = self.current_file.clone();
                    
                    // Get the prompt
                    let prompt = self.current_prompt.clone();
                    
                    // Get the selected files content
                    let selected_files_content = self.selected_files_content.clone();
                    
                    // Get the context from selected files
                    let context = self.get_selected_files_content();
                    
                    // Spawn a new task to handle the API request
                    let prompt_tx = crate::tui::PROMPT_RESULT_TX.lock().unwrap().clone();
                    
                    tokio::spawn(async move {
                        // Determine the language of the current file
                        let language = if let Some(ref path) = current_file {
                            crate::api::get_file_language(path)
                        } else {
                            "plaintext"
                        };
                        
                        // Send the request to the API
                        match crate::api::send_code_modification_request_with_context(
                            &api_key,
                            &prompt,
                            &current_file_content,
                            &context,
                            None, // No AST for now
                            &model,
                            language,
                        ).await {
                            Ok(response) => {
                                // Check if the response contains multiple file edits
                                if (response.contains("File:") || response.contains("file:")) && response.contains("```") {
                                    // Process multi-file response
                                    match crate::api::process_multi_file_response(
                                        response.clone(),
                                        &selected_files_content,
                                    ).await {
                                        Ok(file_edits) => {
                                            if !file_edits.is_empty() {
                                                // Send the multi-file diffs back to the main thread
                                                let _ = prompt_tx.send(crate::tui::PromptResult::MultiFileSuccess(file_edits));
                                                return;
                                            }
                                        }
                                        Err(e) => {
                                            // If multi-file processing fails, fall back to single file
                                            log::error!("Failed to process multi-file response: {}", e);
                                        }
                                    }
                                }
                                
                                // Process single file response
                                if let Some(ref path) = current_file {
                                    match crate::api::process_response(response, &current_file_content, path).await {
                                        Ok(diff) => {
                                            let _ = prompt_tx.send(crate::tui::PromptResult::Success(diff));
                                        }
                                        Err(e) => {
                                            let _ = prompt_tx.send(crate::tui::PromptResult::Error(format!("Failed to process response: {}", e)));
                                        }
                                    }
                                } else {
                                    let _ = prompt_tx.send(crate::tui::PromptResult::Error("No file selected".to_string()));
                                }
                            }
                            Err(e) => {
                                let _ = prompt_tx.send(crate::tui::PromptResult::Error(format!("API request failed: {}", e)));
                            }
                        };
                    });
                    
                    // Clear the prompt
                    self.current_prompt.clear();
                }
            }
            KeyCode::Char(c) => {
                self.current_prompt.push(c);
                // Update token count immediately
                self.update_token_count();
            }
            KeyCode::Backspace => {
                self.current_prompt.pop();
                // Update token count immediately 
                self.update_token_count();
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Apply changes from a multi-file response
    pub fn apply_multi_file_changes(&mut self) -> Result<(), anyhow::Error> {
        // Check if we have multi-file diffs
        let diffs_info = if let Some(ref diffs) = self.multi_file_diffs {
            if diffs.is_empty() {
                self.set_info_message("No changes to apply");
                return Ok(());
            }
            
            // Log what we're trying to apply
            info!("Attempting to apply changes to {} files", diffs.len());
            for (file_name, _) in diffs {
                info!("  - {}", file_name);
            }
            
            // Clone the necessary data to avoid borrow checker issues
            let diffs_count = diffs.len();
            let diffs_clone: Vec<(String, FileDiff)> = diffs.clone();
            Some((diffs_count, diffs_clone))
        } else {
            self.set_info_message("No multi-file changes to apply");
            return Ok(());
        };
        
        // Unwrap the tuple since we know it's Some at this point
        let (diffs_count, diffs) = diffs_info.unwrap();
        let mut success_count = 0;
        let mut error_messages = Vec::new();
        
        // For each file, try to apply the changes
        for (file_name, diff) in diffs {
            // Find the file path
            let file_path = match self.find_file_path(&file_name) {
                Some(path) => {
                    info!("Found existing file path for {}: {:?}", file_name, path);
                    path
                },
                None => {
                    // For new files, create in the current directory
                    let new_path = if file_name.contains('/') || file_name.contains('\\') {
                        // If the file_name has directory components, respect those
                        let path = Path::new(&file_name);
                        if let Some(parent) = path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        path.to_path_buf()
                    } else {
                        self.current_dir.join(&file_name)
                    };
                    
                    info!("Creating new file: {:?}", new_path);
                    // Check if the parent directory exists, and create if not
                    if let Some(parent) = new_path.parent() {
                        if !parent.exists() {
                            info!("Creating parent directory: {:?}", parent);
                            std::fs::create_dir_all(parent)?;
                        }
                    }
                    new_path
                }
            };
            
            // Apply the changes
            let result = std::fs::write(&file_path, &diff.modified);
            
            match result {
                Ok(_) => {
                    info!("Successfully applied changes to {}", file_name);
                    success_count += 1;
                },
                Err(e) => {
                    let error = format!("Failed to create new file {}: {}", file_name, e);
                    error!("{}", error);
                    error_messages.push(error);
                }
            }
        }
        
        // Refresh the file list to show the new files
        self.refresh_file_list()?;
        
        // Set a message based on the results
        if success_count == diffs_count {
            self.set_success_message(&format!(
                "Successfully modified {} file{}",
                success_count,
                if success_count > 1 { "s" } else { "" }
            ));
        } else if success_count > 0 {
            self.set_info_message(&format!(
                "Applied changes to {} file{}, but failed on {} file{}",
                success_count,
                if success_count > 1 { "s" } else { "" },
                diffs_count - success_count,
                if diffs_count - success_count > 1 { "s" } else { "" }
            ));
        } else {
            self.set_error_message(&format!("Failed to apply any changes: {}", error_messages.join(", ")));
        }
        
        Ok(())
    }
    
    /// Find a file path by name
    fn find_file_path(&self, file_name: &str) -> Option<PathBuf> {
        // Log for debugging
        info!("Looking for file path matching: {}", file_name);
        
        // Normalize the file name (remove leading ./ if present)
        let normalized_name = if file_name.starts_with("./") {
            &file_name[2..]
        } else {
            file_name
        };
        
        // First try exact path match in the file list and selected files
        for paths in &[&self.file_list, &self.selected_files] {
            for path in *paths {
                let path_str = path.to_string_lossy().to_string();
                if path_str.ends_with(normalized_name) {
                    info!("Found exact path match: {:?}", path);
                    return Some(path.clone());
                }
            }
        }
        
        // Try exact filename match (just the file name, not path)
        for paths in &[&self.file_list, &self.selected_files] {
            for path in *paths {
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str == normalized_name {
                        info!("Found exact filename match: {:?}", path);
                        return Some(path.clone());
                    }
                }
            }
        }
        
        // Try extracting just the basename from the file_name if it contains path separators
        let basename = if normalized_name.contains('/') || normalized_name.contains('\\') {
            Path::new(normalized_name)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
        } else {
            Some(normalized_name.to_string())
        };
        
        if let Some(basename) = basename {
            // Try basename match
            for paths in &[&self.selected_files, &self.file_list] {
                for path in *paths {
                    if let Some(name) = path.file_name() {
                        if name.to_string_lossy() == basename {
                            info!("Found basename match: {:?}", path);
                            return Some(path.clone());
                        }
                    }
                }
            }
        }
        
        // Try case insensitive match
        let lowercase_name = normalized_name.to_lowercase();
        for paths in &[&self.selected_files, &self.file_list] {
            for path in *paths {
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().to_lowercase() == lowercase_name {
                        info!("Found case-insensitive match: {:?}", path);
                        return Some(path.clone());
                    }
                }
            }
        }
        
        // If the context file is set, try to use its directory for relative paths
        if let Some(ref context_file) = self.current_file {
            if let Some(parent) = context_file.parent() {
                let potential_path = parent.join(normalized_name);
                if potential_path.exists() {
                    info!("Found using context file's directory: {:?}", potential_path);
                    return Some(potential_path);
                }
            }
        }
        
        // No match found
        info!("No matching file found for: {}", file_name);
        None
    }
    
    /// Navigate to the next file diff
    pub fn next_diff_file(&mut self) -> bool {
        if let Some(ref multi_diffs) = self.multi_file_diffs {
            if multi_diffs.is_empty() {
                return false;
            }
            
            // Find the index of the current file
            let current_idx = if let Some(ref current_file) = self.current_diff_file {
                multi_diffs.iter().position(|(file_name, _)| file_name == current_file)
            } else {
                None
            };
            
            // Calculate the next index
            let next_idx = match current_idx {
                Some(idx) if idx + 1 < multi_diffs.len() => idx + 1,
                Some(_) => 0, // Wrap around to the first file
                None => 0,    // Start with the first file
            };
            
            // Set the current diff to the next file
            let (file_name, diff) = &multi_diffs[next_idx];
            self.current_diff_file = Some(file_name.clone());
            self.current_diff = Some(diff.clone());
            
            // Reset scroll position when changing files
            self.scroll_position = 0;
            
            // Log the navigation
            info!("Navigated to next file: {}", file_name);
            
            true
        } else {
            false
        }
    }
    
    /// Navigate to the previous file diff
    pub fn prev_diff_file(&mut self) -> bool {
        if let Some(ref multi_diffs) = self.multi_file_diffs {
            if multi_diffs.is_empty() {
                return false;
            }
            
            // Find the index of the current file
            let current_idx = if let Some(ref current_file) = self.current_diff_file {
                multi_diffs.iter().position(|(file_name, _)| file_name == current_file)
            } else {
                None
            };
            
            // Calculate the previous index
            let prev_idx = match current_idx {
                Some(0) => multi_diffs.len() - 1, // Wrap around to the last file
                Some(idx) => idx - 1,
                None => 0, // Start with the first file
            };
            
            // Set the current diff to the previous file
            let (file_name, diff) = &multi_diffs[prev_idx];
            self.current_diff_file = Some(file_name.clone());
            self.current_diff = Some(diff.clone());
            
            // Reset scroll position when changing files
            self.scroll_position = 0;
            
            // Log the navigation
            info!("Navigated to previous file: {}", file_name);
            
            true
        } else {
            false
        }
    }
    
    /// Toggle between explanation and diff views
    pub fn toggle_explanation_view(&mut self) {
        // Toggle the flag
        self.show_explanation = !self.show_explanation;
        
        // Reset scroll position when toggling views
        self.scroll_position = 0;
        
        // Log the toggle action
        if self.show_explanation {
            debug!("Switched to explanation view");
        } else {
            debug!("Switched to diff view");
        }
    }
    
    /// Handle key events for custom model input
    fn handle_custom_model_input_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Enter => {
                // Save the custom model if it's not empty
                if !self.current_prompt.is_empty() {
                    self.custom_model = self.current_prompt.clone();
                    // Set the selected model to the custom model value
                    self.selected_model = self.current_prompt.clone();
                    let success_message = format!("Custom model '{}' configured and selected", self.current_prompt);
                    self.set_success_message(&success_message);
                    
                    // Save model to config if we have an API key
                    if let Some(_api_key) = &self.api_key {
                        if let Some(_config_dir) = dirs::config_dir() {
                            let _ = crate::api::save_preferred_model(&self.selected_model);
                        }
                    }
                }
                
                // Return to configuration screen
                self.mode = AppMode::Configuration;
            }
            KeyCode::Esc => {
                // Cancel and return to configuration screen
                self.mode = AppMode::Configuration;
            }
            KeyCode::Char(c) => {
                // Add character to custom model
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

    /// Log the current state of the diff viewer for debugging
    fn log_diff_viewer_state(&self) {
        if let Some(ref diff_file) = self.current_diff_file {
            info!("Current diff file: {}", diff_file);
        } else {
            info!("No current diff file");
        }
        
        if let Some(ref diff) = self.current_diff {
            info!("Current diff: {} original lines, {} modified lines", 
                  diff.original.lines().count(),
                  diff.modified.lines().count());
            
            if let Some(ref explanation) = diff.explanation_text {
                info!("Explanation text available: {} chars", explanation.len());
            } else {
                info!("No explanation text in current diff");
            }
        } else {
            info!("No current diff");
        }
        
        if let Some(ref multi_diffs) = self.multi_file_diffs {
            info!("Multi-file diffs: {} files", multi_diffs.len());
            for (file_name, _) in multi_diffs {
                info!("  - {}", file_name);
            }
        } else {
            info!("No multi-file diffs");
        }
        
        info!("Active panel: {:?}", self.active_panel);
        info!("Scroll position: {}", self.scroll_position);
        info!("Show explanation: {}", self.show_explanation);
    }

    /// Count tokens in a string (approximation)
    pub fn count_tokens(&self, text: &str) -> usize {
        // A more accurate token estimation that considers code structure
        // For code, we count tokens more conservatively
        
        // First, count the characters
        let char_count = text.chars().count();
        
        // Different languages tokenize differently, but a reasonable estimation
        // is 4-6 characters per token for code
        const CHARS_PER_TOKEN: f32 = 4.0;
        
        // Calculate tokens and add a 10% margin for safety
        let token_estimate = (char_count as f32 / CHARS_PER_TOKEN).ceil() as usize;
        let with_margin = (token_estimate as f32 * 1.1).ceil() as usize;
        
        // Log token estimates for debugging
        debug!("Token estimate for text of {} chars: {} tokens (with margin: {})",
               char_count, token_estimate, with_margin);
        
        with_margin
    }
    
    /// Update token count based on selected files and prompt
    pub fn update_token_count(&mut self) {
        let mut total_tokens = 0;
        
        // Count tokens in selected files
        for (_, content) in &self.selected_files_content {
            let file_tokens = self.count_tokens(content);
            debug!("File tokens: {}", file_tokens);
            total_tokens += file_tokens;
        }
        
        // Count tokens in the prompt
        let prompt_tokens = self.count_tokens(&self.current_prompt);
        debug!("Prompt tokens: {}", prompt_tokens);
        total_tokens += prompt_tokens;
        
        // Add a fixed overhead for system prompts and formatting
        const SYSTEM_PROMPT_OVERHEAD: usize = 1000;
        total_tokens += SYSTEM_PROMPT_OVERHEAD;
        
        // Update the token count
        self.token_count = total_tokens;
        debug!("Total token count updated: {}", self.token_count);
    }
    
    /// Update the context window size based on the selected model
    pub fn update_context_window_size(&mut self) {
        // Set context window size based on the selected model
        if self.selected_model.contains("gemini") {
            self.context_window_size = 1_000_000; // 1M tokens for Gemini
        } else if self.selected_model.contains("claude") {
            self.context_window_size = 200_000; // 200K tokens for Claude
        } else if self.selected_model.contains("custom") {
            // For custom models, use a conservative default
            self.context_window_size = 100_000;
        } else {
            // Default to a conservative estimate for other models
            self.context_window_size = 100_000;
        }
        
        // Log the context window size
        debug!("Context window size set to {} tokens for model {}", 
               self.context_window_size, self.selected_model);
    }
}