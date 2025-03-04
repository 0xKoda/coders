use crate::app::{App, AppMode, MessageType, ProcessingState};
use std::path::Path;
use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap, Gauge, ListState},
    layout::{Layout, Constraint, Direction, Alignment, Rect},
    Frame,
    backend::CrosstermBackend,
    Terminal,
};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

// Color theme
const PRIMARY_COLOR: Color = Color::Cyan;
const SECONDARY_COLOR: Color = Color::Yellow;
const ACCENT_COLOR: Color = Color::Rgb(147, 112, 219);      // Medium Purple
const HIGHLIGHT_COLOR: Color = Color::Green;
const BG_COLOR: Color = Color::Rgb(25, 25, 40);            // Dark Blue-Grey
const SUCCESS_COLOR: Color = Color::Green;
const ERROR_COLOR: Color = Color::Red;
const INFO_COLOR: Color = Color::Blue;

/// Render the main UI
pub fn render(f: &mut Frame, app: &App) {
    // Create a layout for the main UI components
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Status bar
            Constraint::Min(1),     // Main content
            Constraint::Length(1),  // Message bar (only shown if there's a message)
            Constraint::Length(1),  // Command bar
        ])
        .split(f.size());
    
    // Render the appropriate screen based on the current mode
    match app.mode {
        AppMode::Welcome => render_welcome(f, app, chunks[1]),
        AppMode::Configuration => render_config(f, app, chunks[1]),
        AppMode::ApiKeyInput => render_api_key_input(f, app, chunks[1]),
        AppMode::CustomModelInput => render_custom_model_input(f, app, chunks[1]),
        AppMode::FileBrowser => render_file_browser(f, app, chunks[1]),
        AppMode::FileSelection => render_file_selection(f, app, chunks[1]),
        AppMode::Editor => render_editor(f, app, chunks[1]),
        AppMode::PromptInput => render_prompt_input(f, app, chunks[1]),
        AppMode::Results => render_results(f, app, chunks[1]),
        AppMode::Help => render_help(f, app, chunks[1]),
        AppMode::Credits => render_credits_screen(f, app, chunks[1]),
    }
    
    // Render message bar if there's a message
    if app.message.is_some() {
        render_message_bar(f, app, "", chunks[2]);
    }
    
    // Render command bar
    render_command_bar(f, app, chunks[3]);
    
    // Render credits overlay if enabled (regardless of the current mode)
    if app.show_credits && app.mode != AppMode::Credits {
        render_credits(f, app, f.size());
    }
}

/// Render the welcome screen
pub fn render_welcome(f: &mut Frame, _app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue))
        .title(" Welcome ");
    
    let inner_area = block.inner(area);
    
    let welcome_text = vec![
        Line::from(vec![
            Span::styled("Welcome", Style::default().fg(Color::Blue)),
        ]),
        Line::from(vec![
            Span::styled("Code-AI Assistant", Style::default().fg(Color::Magenta)),
            Span::raw(" - "),
            Span::raw("An AI-powered code editing assistant"),
        ]),
        Line::from(vec![
            Span::styled("Press ", Style::default()),
            Span::styled("ENTER", Style::default().fg(Color::Magenta)),
            Span::raw(" to start"),
        ]),
        Line::from(vec![
            Span::styled("Press ", Style::default()),
            Span::styled("q", Style::default().fg(Color::Magenta)),
            Span::raw(" to quit"),
        ]),
    ];
    
    let welcome_paragraph = Paragraph::new(welcome_text)
        .block(Block::default())
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    
    f.render_widget(block, area);
    f.render_widget(welcome_paragraph, inner_area);
}

/// Renders the configuration screen
pub fn render_config(f: &mut Frame, app: &App, area: Rect) {
    // Split the area into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(3),  // API Key
            Constraint::Length(1),  // Spacer
            Constraint::Length(2),  // Model selection title
            Constraint::Min(5),     // Model selection list
            Constraint::Length(3),  // Instructions
        ])
        .split(area);
    
    // Title
    let title = Paragraph::new("Configuration")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);
    
    // API Key
    let api_key_text = if let Some(key) = &app.api_key {
        format!("API Key: {}", mask_api_key(key))
    } else {
        "API Key: Not configured".to_string()
    };
    
    let api_key = Paragraph::new(api_key_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("API Key"));
    f.render_widget(api_key, chunks[1]);
    
    // Model selection title
    let model_title = Paragraph::new("Select a model:")
        .style(Style::default().fg(Color::White));
    f.render_widget(model_title, chunks[3]);
    
    // Model selection list
    let models: Vec<ListItem> = app.available_models
        .iter()
        .map(|m| {
            let display_name = if m == "custom" {
                if app.custom_model.is_empty() {
                    "Custom model (not set)".to_string()
                } else {
                    format!("Custom model: {}", app.custom_model)
                }
            } else {
                m.clone()
            };
            
            let style = if m == &app.selected_model || 
                        (m == "custom" && app.selected_model == app.custom_model && !app.custom_model.is_empty()) {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            
            ListItem::new(display_name).style(style)
        })
        .collect();
    
    let models_list = List::new(models)
        .block(Block::default().borders(Borders::ALL).title("Available Models"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_widget(models_list, chunks[4]);
    
    // Instructions
    let instructions = if app.api_key.is_some() {
        "Press Enter to continue, q to go back, Ctrl+A to change API key"
    } else {
        "Press Enter to set API key, q to go back"
    };
    
    // Add custom model instructions if "custom" is selected
    let instructions = if app.selected_model == "custom" {
        "Press Enter to input custom model, q to go back"
    } else {
        instructions
    };
    
    let instructions_widget = Paragraph::new(instructions)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);
    f.render_widget(instructions_widget, chunks[5]);
}

/// Renders the API key input screen
fn render_api_key_input(f: &mut Frame, app: &App, _area: Rect) {
    let area = f.size();
    
    // Create a centered area for the API key input
    let input_area = centered_rect(60, 20, area);
    
    // Split the input area into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(3),  // Instructions
            Constraint::Length(3),  // Input field
            Constraint::Min(0),     // Spacer
        ].as_ref())
        .split(input_area);
    
    // Title
    let title = Paragraph::new("Enter OpenRouter API Key")
        .style(Style::default().fg(PRIMARY_COLOR).add_modifier(Modifier::BOLD))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(title, chunks[0]);
    
    // Instructions
    let instructions = Paragraph::new("Your API key will be saved securely in your config directory")
        .style(Style::default().fg(SECONDARY_COLOR))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(instructions, chunks[1]);
    
    // Input field
    let input = Paragraph::new(app.current_prompt.as_str())
        .style(Style::default().fg(HIGHLIGHT_COLOR))
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT_COLOR))
            .title("API Key"))
        .alignment(ratatui::layout::Alignment::Left);
    f.render_widget(input, chunks[2]);
    
    // Calculate cursor position correctly
    // The cursor should be positioned at the start of the input area + the length of the prompt
    // We need to account for the border and padding
    let cursor_x = chunks[2].x + 1 + app.current_prompt.len() as u16;
    let cursor_y = chunks[2].y + 1; // Position at the first line inside the block
    
    // Set cursor position
    f.set_cursor(cursor_x, cursor_y);
}

/// Renders the file browser
fn render_file_browser(f: &mut Frame, app: &App, _area: Rect) {
    let area = f.size();
    
    // Split the screen into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Title with current directory
            Constraint::Min(1),     // File list
            Constraint::Length(5),  // Selected files summary
            Constraint::Length(3),  // Instructions
        ].as_ref())
        .split(area);
    
    // Title with current directory
    let current_dir = app.current_dir.to_string_lossy();
    let title = Paragraph::new(format!("Directory: {}", current_dir))
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(PRIMARY_COLOR));
    f.render_widget(title, chunks[0]);
    
    // File list
    let items: Vec<ListItem> = app.file_list
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let is_selected = i == app.selected_file_idx;
            let is_dir = path.is_dir();
            let is_file_selected = !is_dir && app.selected_files.contains(path);
            
            let name = if path.file_name().map_or(false, |name| name == "..") {
                "[Parent Directory]".to_string()
            } else {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "[Unknown]".to_string())
            };
            
            let display_name = if is_dir {
                format!("📁 {}/", name)
            } else if is_file_selected {
                format!("✓ 📄 {}", name)
            } else {
                format!("📄 {}", name)
            };
            
            let style = if is_selected {
                Style::default().fg(HIGHLIGHT_COLOR).add_modifier(Modifier::BOLD)
            } else if is_file_selected {
                Style::default().fg(SUCCESS_COLOR)
            } else if is_dir {
                Style::default().fg(SECONDARY_COLOR)
            } else {
                Style::default().fg(Color::White)
            };
            
            ListItem::new(Line::from(Span::styled(display_name, style)))
        })
        .collect();
    
    let files_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Files"))
        .highlight_style(Style::default().fg(HIGHLIGHT_COLOR).add_modifier(Modifier::BOLD));
    
    f.render_widget(files_list, chunks[1]);
    
    // Selected files summary
    let selected_files_text = if app.selected_files.is_empty() {
        vec![Line::from(Span::styled(
            "No files selected. Use 's' to select files for context.",
            Style::default().fg(SECONDARY_COLOR)
        ))]
    } else {
        let mut lines = vec![Line::from(Span::styled(
            format!("{} files selected for context:", app.selected_files.len()),
            Style::default().fg(SUCCESS_COLOR).add_modifier(Modifier::BOLD)
        ))];
        
        // Show up to 3 selected files with ellipsis if more
        for (i, path) in app.selected_files.iter().take(3).enumerate() {
            if let Some(file_name) = path.file_name() {
                lines.push(Line::from(Span::styled(
                    format!("- {}", file_name.to_string_lossy()),
                    Style::default().fg(ACCENT_COLOR)
                )));
            }
            
            // Add ellipsis if there are more files
            if i == 2 && app.selected_files.len() > 3 {
                lines.push(Line::from(Span::styled(
                    format!("... and {} more", app.selected_files.len() - 3),
                    Style::default().fg(ACCENT_COLOR)
                )));
            }
        }
        
        lines
    };
    
    let selected_files = Paragraph::new(selected_files_text)
        .block(Block::default().borders(Borders::ALL).title("Selected Files"))
        .style(Style::default().fg(Color::White));
    
    f.render_widget(selected_files, chunks[2]);
    
    // Instructions
    let instructions = Paragraph::new("[↑/↓] Navigate | [Enter] Open | [S] Select file | [P] Prompt with selected | [L] List selected | [Shift+C] Clear")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(SECONDARY_COLOR));
    f.render_widget(instructions, chunks[3]);
}

/// Renders the file selection screen
fn render_file_selection(f: &mut Frame, app: &App, _area: Rect) {
    let area = f.size();
    
    // Split the screen into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Title with current directory
            Constraint::Min(1),     // File list
            Constraint::Length(5),  // Selected files
            Constraint::Length(3),  // Token count progress bar
            Constraint::Length(3),  // Instructions
        ].as_ref())
        .split(area);
    
    // Title with current directory
    let current_dir = app.current_dir.to_string_lossy();
    let title = Paragraph::new(format!("Directory: {}", current_dir))
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(PRIMARY_COLOR));
    f.render_widget(title, chunks[0]);
    
    // File list
    let items: Vec<ListItem> = app.file_list
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let is_selected = i == app.selected_file_idx;
            let is_dir = path.is_dir();
            let is_file_selected = !is_dir && app.selected_files.contains(path);
            
            let name = if path.file_name().map_or(false, |name| name == "..") {
                "[Parent Directory]".to_string()
            } else {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "[Unknown]".to_string())
            };
            
            let prefix = if is_dir {
                "📁 "
            } else {
                "📄 "
            };
            
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(PRIMARY_COLOR)
            } else if is_file_selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            
            ListItem::new(format!("{}{}", prefix, name)).style(style)
        })
        .collect();
    
    let file_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Files "))
        .highlight_style(Style::default().fg(Color::Black).bg(PRIMARY_COLOR));
    
    f.render_stateful_widget(file_list, chunks[1], &mut ListState::default().with_selected(Some(app.selected_file_idx)));
    
    // Selected files
    let selected_files_text = if app.selected_files.is_empty() {
        vec![
            Line::from("No files selected for context"),
        ]
    } else {
        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("{} files selected for context:", app.selected_files.len()),
                    Style::default().fg(Color::Green)
                ),
            ]),
        ];
        
        for (i, path) in app.selected_files.iter().take(3).enumerate() {
            let file_name = path.file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "[Unknown]".to_string());
            
            lines.push(Line::from(format!("  {}. {}", i + 1, file_name)));
        }
        
        if app.selected_files.len() > 3 {
            lines.push(Line::from(
                format!("  ... and {} more", app.selected_files.len() - 3),
            ));
        }
        
        lines
    };
    
    let selected_files = Paragraph::new(selected_files_text)
        .block(Block::default().borders(Borders::ALL).title(" Selected Files "))
        .wrap(Wrap { trim: true });
    
    f.render_widget(selected_files, chunks[2]);
    
    // Token count progress bar
    let token_percentage = if app.context_window_size > 0 {
        (app.token_count as f64 / app.context_window_size as f64 * 100.0).min(100.0)
    } else {
        0.0
    };
    
    let token_count_text = format!(
        "Context usage: {}/{} tokens ({:.1}%)",
        app.token_count,
        app.context_window_size,
        token_percentage
    );
    
    let progress_color = if token_percentage < 50.0 {
        Color::Green
    } else if token_percentage < 80.0 {
        Color::Yellow
    } else {
        Color::Red
    };
    
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" Context Window Usage "))
        .gauge_style(Style::default().fg(progress_color))
        .ratio(token_percentage / 100.0)
        .label(token_count_text);
    
    f.render_widget(gauge, chunks[3]);
    
    // Instructions
    let instructions = Paragraph::new(
        "↑/↓: Navigate | Enter: Open | Space: Select/Deselect | Esc: Back | p: Proceed with selected files"
    )
    .block(Block::default().borders(Borders::ALL))
    .style(Style::default().fg(Color::Gray));
    
    f.render_widget(instructions, chunks[4]);
}

/// Renders the code editor
fn render_editor(f: &mut Frame, app: &App, area: Rect) {
    // Split the screen into sections
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // File info
            Constraint::Min(1),     // Code viewer and sidebar
            Constraint::Length(3),  // Status bar
        ].as_ref())
        .split(area);
    
    // File info
    let file_name = app.current_file
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "[No File]".to_string());
    
    let language = app.current_file
        .as_ref()
        .map(|p| get_file_language(p))
        .unwrap_or("unknown");
    
    let title = Paragraph::new(format!("File: {} ({})", file_name, language))
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(PRIMARY_COLOR));
    f.render_widget(title, main_chunks[0]);
    
    // Split the main area into code viewer and sidebar
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(75),  // Code viewer
            Constraint::Percentage(25),  // Sidebar
        ].as_ref())
        .split(main_chunks[1]);
    
    // Code content
    let mut lines = Text::default();
    
    for (i, line) in app.current_file_content.lines().enumerate() {
        lines.lines.push(Line::from(vec![
            Span::styled(
                format!("{:4} ", i + 1),
                Style::default().fg(SECONDARY_COLOR)
            ),
            Span::raw(line),
        ]));
    }
    
    let code_paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL))
        .scroll((app.scroll_position as u16, 0));
    
    f.render_widget(code_paragraph, content_chunks[0]);
    
    // Sidebar with selected model and files
    render_sidebar(f, app, content_chunks[1]);
    
    // Status bar
    let status_text = format!(
        "Mode: {} | File: {} | Model: {}",
        format!("{:?}", app.mode),
        file_name,
        app.selected_model
    );
    
    let status_bar = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(SECONDARY_COLOR));
    
    f.render_widget(status_bar, main_chunks[2]);
    
    // Credits info is now handled by the main render function
    // We don't need to render message bar here as it's handled by the main render function
}

/// Render the sidebar with model info and selected files
fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Model info
            Constraint::Min(1),     // Selected files
        ].as_ref())
        .split(area);
    
    // Model info
    let model_text = format!("Model: {}", app.selected_model);
    let model_paragraph = Paragraph::new(model_text)
        .block(Block::default().borders(Borders::ALL).title("Configuration"))
        .style(Style::default().fg(SECONDARY_COLOR));
    
    f.render_widget(model_paragraph, chunks[0]);
    
    // Selected files
    let selected_files_text = if app.selected_files.is_empty() {
        "No files selected for context".to_string()
    } else {
        let mut text = format!("{} files selected:\n", app.selected_files.len());
        for file_path in &app.selected_files {
            let file_name = Path::new(file_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            text.push_str(&format!("- {}\n", file_name));
        }
        text
    };
    
    let selected_files_paragraph = Paragraph::new(selected_files_text)
        .block(Block::default().borders(Borders::ALL).title("Selected Files"))
        .style(Style::default().fg(ACCENT_COLOR))
        .wrap(Wrap { trim: true });
    
    f.render_widget(selected_files_paragraph, chunks[1]);
}

/// Render the message bar
fn render_message_bar(f: &mut Frame, app: &App, message: &str, area: Rect) {
    let style = match app.message_type {
        MessageType::Info => Style::default().fg(Color::Blue),
        MessageType::Error => Style::default().fg(Color::Red),
        MessageType::Success => Style::default().fg(Color::Green),
    };
    
    // Create a paragraph with the message
    let message_paragraph = Paragraph::new(message.to_string())
        .style(style)
        .wrap(Wrap { trim: true });
    
    f.render_widget(message_paragraph, area);
}

/// Render credits information
fn render_credits(f: &mut Frame, app: &App, _area: Rect) {
    if app.show_credits {
        if let Some(credits_info) = &app.credits_info {
            // Format the credits information
            let remaining = credits_info.total_credits - credits_info.total_usage;
            let percentage_used = if credits_info.total_credits > 0.0 {
                (credits_info.total_usage / credits_info.total_credits) * 100.0
            } else {
                0.0
            };
            
            // Create a block for the credits information
            let block = Block::default()
                .title("OpenRouter Credits")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            
            // Create the text for the credits information
            let text = vec![
                Line::from(vec![
                    Span::styled("Total Credits: ", Style::default().fg(Color::Green)),
                    Span::raw(format!("${:.2}", credits_info.total_credits)),
                ]),
                Line::from(vec![
                    Span::styled("Used: ", Style::default().fg(Color::Yellow)),
                    Span::raw(format!("${:.2} ({:.1}%)", credits_info.total_usage, percentage_used)),
                ]),
                Line::from(vec![
                    Span::styled("Remaining: ", Style::default().fg(Color::Blue)),
                    Span::raw(format!("${:.2}", remaining)),
                ]),
                Line::from(vec![
                    Span::styled("Last Updated: ", Style::default().fg(Color::Gray)),
                    Span::raw(format_system_time(credits_info.last_updated)),
                ]),
            ];
            
            // Create a paragraph with the text
            let paragraph = Paragraph::new(text)
                .block(block)
                .wrap(Wrap { trim: true });
            
            // Calculate the area for the credits information
            // Use a small popup in the corner, ensuring it stays within bounds
            let width = 40.min(f.size().width.saturating_sub(2));
            let height = 6.min(f.size().height.saturating_sub(2));
            
            // Ensure we don't position outside the screen bounds
            let x = f.size().width.saturating_sub(width + 1);
            let y = 1; // Position at the top with a small margin
            
            let credits_area = Rect {
                x,
                y,
                width,
                height,
            };
            
            // Render the paragraph
            f.render_widget(paragraph, credits_area);
        }
    }
}

/// Format a SystemTime for display
fn format_system_time(time: std::time::SystemTime) -> String {
    match time.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => {
            let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(
                duration.as_secs() as i64,
                duration.subsec_nanos(),
            );
            
            if let Some(dt) = datetime {
                dt.format("%Y-%m-%d %H:%M:%S").to_string()
            } else {
                "Invalid time".to_string()
            }
        }
        Err(_) => "Invalid time".to_string(),
    }
}

/// Render the prompt input screen
pub fn render_prompt_input(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue))
        .title(" Prompt Input ");
    
    let inner_area = block.inner(area);
    
    // Split the area into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),      // Prompt input area
            Constraint::Length(3),   // Token count progress bar
            Constraint::Length(3),   // Instructions
        ])
        .split(inner_area);
    
    // Determine if we're processing or waiting for input
    if app.processing_state == ProcessingState::Processing {
        // Show a "cooking" animation when processing
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner = spinner_chars[app.spinner_frame % spinner_chars.len()];
        
        // Select just one cooking message based on spinner frame
        let cooking_messages = [
            "Thinking...",
            "Processing your request...",
            "Analyzing code...",
            "Generating response...",
            "Cooking up some code...",
            "Brewing a solution...",
            "Crunching algorithms...",
            "Consulting the AI oracle...",
        ];
        
        // Use a slower rotation for messages - only change every 10 frames
        let message_index = (app.spinner_frame / 10) % cooking_messages.len();
        let cooking_message = cooking_messages[message_index];
        
        let text = vec![
            Line::from(vec![
                Span::styled(format!(" {} {} ", spinner, cooking_message), 
                           Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::raw(""),
            ]),
        ];
        
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::NONE))
            .alignment(Alignment::Center);
        
        f.render_widget(paragraph, chunks[0]);
    } else {
        // Show the prompt input area
        let prompt_text = if app.current_prompt.is_empty() {
            vec![
                Line::from(vec![
                    Span::styled(
                        "Enter your prompt here. Describe what you want to do with the code.",
                        Style::default().fg(Color::DarkGray)
                    ),
                ]),
            ]
        } else {
            // Split the prompt into lines for proper wrapping
            app.current_prompt.lines()
                .map(|line| Line::from(line.to_string()))
                .collect()
        };
        
        let prompt_paragraph = Paragraph::new(prompt_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true });
        
        f.render_widget(prompt_paragraph, chunks[0]);
        
        // Calculate cursor position
        if app.mode == AppMode::PromptInput && app.processing_state == ProcessingState::Idle {
            // Count lines and characters to determine cursor position
            let lines: Vec<&str> = app.current_prompt.split('\n').collect();
            let line_count = lines.len();
            
            if line_count > 0 {
                let last_line = lines[line_count - 1];
                let last_line_width = last_line.len() as u16;
                
                // Calculate cursor position within the visible area
                let x = last_line_width.min(chunks[0].width.saturating_sub(1));
                let y = (line_count as u16 - 1).min(chunks[0].height.saturating_sub(1));
                
                // Set cursor position
                f.set_cursor(
                    chunks[0].x + x,
                    chunks[0].y + y
                );
            } else {
                // Default cursor position at the beginning
                f.set_cursor(chunks[0].x, chunks[0].y);
            }
        }
    }
    
    // Token count progress bar
    let token_percentage = if app.context_window_size > 0 {
        (app.token_count as f64 / app.context_window_size as f64 * 100.0).min(100.0)
    } else {
        0.0
    };
    
    let token_count_text = format!(
        "Context usage: {}/{} tokens ({:.1}%)",
        app.token_count,
        app.context_window_size,
        token_percentage
    );
    
    let progress_color = if token_percentage < 50.0 {
        Color::Green
    } else if token_percentage < 80.0 {
        Color::Yellow
    } else {
        Color::Red
    };
    
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" Context Window Usage "))
        .gauge_style(Style::default().fg(progress_color))
        .ratio(token_percentage / 100.0)
        .label(token_count_text);
    
    f.render_widget(gauge, chunks[1]);
    
    // Instructions
    let instructions = if !app.selected_files.is_empty() {
        "Enter: Submit | Esc: Back to file selection"
    } else {
        "Enter: Submit | Esc: Back to editor"
    };
    
    let instructions_paragraph = Paragraph::new(instructions)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    
    f.render_widget(instructions_paragraph, chunks[2]);
    
    f.render_widget(block, area);
}

/// Render a spinner overlay
pub fn render_spinner_overlay(f: &mut Frame, app: &App, area: Rect) {
    // Only show the spinner overlay if we're not in prompt input mode
    // (since prompt input has its own spinner)
    if app.mode == AppMode::PromptInput {
        return;
    }
    
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = spinner_chars[app.spinner_frame % spinner_chars.len()];
    
    let processing_messages = [
        "Processing...",
        "Working on it...",
        "Thinking...",
        "Analyzing...",
        "Computing...",
        "Generating...",
    ];
    
    let message = processing_messages[app.spinner_frame % processing_messages.len()];
    
    // Create a centered box for the spinner
    let width = 40;
    let height = 3;
    let spinner_area = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    
    let spinner_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black));
    
    let spinner_text = Paragraph::new(format!("{} {}", spinner, message))
        .block(spinner_block)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow));
    
    f.render_widget(Clear, spinner_area); // Clear the area first
    f.render_widget(spinner_text, spinner_area);
}

/// Render the results view
pub fn render_results(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(5),    // Content
            Constraint::Length(3), // Help
        ])
        .split(area);

    // Render title
    let title = if let Some(ref file_name) = app.current_diff_file {
        format!(" Diff for {} ", file_name)
    } else {
        " Diff ".to_string()
    };
    
    let title_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(PRIMARY_COLOR));
    f.render_widget(title_block, chunks[0]);

    // Render content
    if app.show_explanation {
        // Show explanation text if available
        let explanation_text = if let Some(ref diff) = app.current_diff {
            diff.explanation_text.as_deref().unwrap_or("No explanation available.")
        } else if let Some(ref explanation) = app.explanation_text {
            explanation
        } else {
            "No explanation available."
        };
        
        let explanation_paragraph = Paragraph::new(explanation_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(" Explanation ")
                .style(Style::default().fg(PRIMARY_COLOR)))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        
        f.render_widget(explanation_paragraph, chunks[1]);
    } else {
        // Show diff view
        let diff_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        if let Some(ref diff) = app.current_diff {
            // Original code panel
            let original_title = match app.active_panel {
                crate::app::ActivePanel::Left => " Original (ACTIVE) ",
                _ => " Original ",
            };
            
            let original_block = Block::default()
                .title(original_title)
                .borders(Borders::ALL)
                .border_type(if app.active_panel == crate::app::ActivePanel::Left {
                    BorderType::Double
                } else {
                    BorderType::Rounded
                })
                .style(Style::default().fg(if app.active_panel == crate::app::ActivePanel::Left {
                    SECONDARY_COLOR
                } else {
                    PRIMARY_COLOR
                }));
            
            let original_text = Paragraph::new(diff.original.clone())
                .block(original_block)
                .style(Style::default().fg(Color::White))
                .scroll((app.scroll_position as u16, 0));
            
            f.render_widget(original_text, diff_chunks[0]);

            // Modified code panel
            let modified_title = match app.active_panel {
                crate::app::ActivePanel::Right => " Modified (ACTIVE) ",
                _ => " Modified ",
            };
            
            let modified_block = Block::default()
                .title(modified_title)
                .borders(Borders::ALL)
                .border_type(if app.active_panel == crate::app::ActivePanel::Right {
                    BorderType::Double
                } else {
                    BorderType::Rounded
                })
                .style(Style::default().fg(if app.active_panel == crate::app::ActivePanel::Right {
                    SECONDARY_COLOR
                } else {
                    PRIMARY_COLOR
                }));
            
            let modified_text = Paragraph::new(diff.modified.clone())
                .block(modified_block)
                .style(Style::default().fg(Color::White))
                .scroll((app.scroll_position as u16, 0));
            
            f.render_widget(modified_text, diff_chunks[1]);
        } else {
            // No diff available
            let no_diff_block = Block::default()
                .title(" No Diff Available ")
                .borders(Borders::ALL)
                .style(Style::default().fg(PRIMARY_COLOR));
            
            f.render_widget(no_diff_block, chunks[1]);
        }
    }

    // Render help
    let help_text = if app.show_explanation {
        "Press [e] to show diff | [q] to quit | [y] to accept | [n] to reject"
    } else {
        "Press [e] to show explanation | [Tab] to switch panels | [←/→] to navigate files | [↑/↓] to scroll | [y] to accept | [n] to reject | [q] to quit"
    };
    
    let help_paragraph = Paragraph::new(help_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" Help ")
            .style(Style::default().fg(PRIMARY_COLOR)))
        .style(Style::default().fg(SECONDARY_COLOR))
        .alignment(Alignment::Center);
    
    f.render_widget(help_paragraph, chunks[2]);
}

/// Renders the help screen
fn render_help(f: &mut Frame, _app: &App, _area: Rect) {
    let area = f.size();
    
    // Create a centered area for the help content
    let help_area = centered_rect(70, 70, area);
    
    // Create the help text with Lines instead of Vec<Span>
    let help_text = vec![
        Line::from(vec![Span::styled(
            "Code-AI Assistant Help",
            Style::default()
                .fg(HIGHLIGHT_COLOR)
                .add_modifier(Modifier::BOLD)
        )]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![Span::styled("Global Commands:", Style::default().fg(PRIMARY_COLOR).add_modifier(Modifier::BOLD))]),
        Line::from("  [Q] - Quit the current screen or application"),
        Line::from("  [H] - Show this help screen"),
        Line::from(""),
        Line::from(vec![Span::styled("File Browser:", Style::default().fg(PRIMARY_COLOR).add_modifier(Modifier::BOLD))]),
        Line::from("  [↑/↓] - Navigate files"),
        Line::from("  [Enter] - Open file or directory"),
        Line::from("  [Backspace] - Go to parent directory"),
        Line::from(""),
        Line::from(vec![Span::styled("Code Editor:", Style::default().fg(PRIMARY_COLOR).add_modifier(Modifier::BOLD))]),
        Line::from("  [↑/↓] - Scroll code"),
        Line::from("  [P] - Enter prompt mode"),
        Line::from("  [B] - Return to file browser"),
        Line::from("  [C] - Toggle credits display"),
        Line::from("  [S] - Toggle selection of current file for multi-file context"),
        Line::from("  [Shift+C] - Clear all selected files"),
        Line::from("  [R] - Refresh OpenRouter credits information"),
        Line::from(""),
        Line::from(vec![Span::styled("Prompt Mode:", Style::default().fg(PRIMARY_COLOR).add_modifier(Modifier::BOLD))]),
        Line::from("  [Enter] - Submit prompt"),
        Line::from("  [Esc] - Cancel and return to editor"),
        Line::from(""),
        Line::from(vec![Span::styled("Results View:", Style::default().fg(PRIMARY_COLOR).add_modifier(Modifier::BOLD))]),
        Line::from("  [Y] - Apply changes"),
        Line::from("  [N] - Discard changes"),
        Line::from("  [E] - Toggle between explanation text and code diff"),
        Line::from("  [←/→] - Switch between original and modified code panels"),
        Line::from("  [Tab/Shift+Tab] - Navigate between files (for multi-file changes)"),
        Line::from("  [↑/↓] - Scroll through code or explanation"),
        Line::from(""),
        Line::from(vec![Span::styled("Multi-File Context:", Style::default().fg(PRIMARY_COLOR).add_modifier(Modifier::BOLD))]),
        Line::from("  Use [S] in editor mode to select files for context"),
        Line::from("  Selected files will be included as context when submitting prompts"),
        Line::from("  This helps the AI understand related code across multiple files"),
    ];
    
    let help_paragraph = Paragraph::new(help_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(PRIMARY_COLOR))
        .title("Help")
        .title_style(Style::default().fg(PRIMARY_COLOR).add_modifier(Modifier::BOLD)))
        .style(Style::default())
        .wrap(Wrap { trim: true });
    
    f.render_widget(help_paragraph, help_area);
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ].as_ref())
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ].as_ref())
        .split(popup_layout[1])[1]
}

/// Helper function to determine file language based on extension
fn get_file_language(file_path: &Path) -> &'static str {
    let extension = file_path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("");

    match extension {
        "js" => "javascript",
        "ts" => "typescript",
        "py" => "python",
        "rs" => "rust",
        "go" => "go",
        "java" => "java",
        "cpp" | "cc" | "cxx" => "c++",
        "c" => "c",
        "cs" => "c#",
        "php" => "php",
        "rb" => "ruby",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "scala" => "scala",
        "hs" => "haskell",
        "lua" => "lua",
        "pl" => "perl",
        "r" => "r",
        "sh" => "shell",
        "sql" => "sql",
        "html" => "html",
        "css" => "css",
        "md" | "markdown" => "markdown",
        "json" => "json",
        "xml" => "xml",
        "yaml" | "yml" => "yaml",
        _ => "plaintext",
    }
}

/// Mask API key for display, showing only first 4 and last 4 characters
fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "****".to_string();
    }
    
    let visible_chars = 4;
    let first = &key[0..visible_chars];
    let last = &key[key.len() - visible_chars..];
    format!("{}****{}", first, last)
}

/// Render the status bar
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_text = format!(
        "Mode: {:?} | Model: {}",
        app.mode, app.selected_model
    );
    
    let status_bar = Paragraph::new(status_text)
        .style(Style::default().fg(SECONDARY_COLOR));
    
    f.render_widget(status_bar, area);
}

/// Render a spinner for processing state
fn render_spinner(f: &mut Frame, app: &App, area: Rect) {
    // Show a spinner in the center of the screen
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = spinner_chars[app.spinner_frame % spinner_chars.len()];
    
    let spinner_area = Rect {
        x: area.width / 2 - 15,
        y: area.height / 2 - 1,
        width: 30,
        height: 3,
    };
    
    let spinner_text = format!("{} Cooking...", spinner);
    let spinner_widget = Paragraph::new(spinner_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(HIGHLIGHT_COLOR))
        .alignment(ratatui::layout::Alignment::Center);
    
    f.render_widget(spinner_widget, spinner_area);
}

/// Render the credits screen
fn render_credits_screen(f: &mut Frame, app: &App, area: Rect) {
    // Split the screen into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(10),    // Credits info
            Constraint::Length(3),  // Instructions
        ].as_ref())
        .split(area);
    
    // Title
    let title = Paragraph::new("OpenRouter Credits Information")
        .style(Style::default().fg(PRIMARY_COLOR).add_modifier(Modifier::BOLD))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(title, chunks[0]);
    
    // Credits information
    if let Some(credits_info) = &app.credits_info {
        let remaining = credits_info.total_credits - credits_info.total_usage;
        let percentage_used = if credits_info.total_credits > 0.0 {
            (credits_info.total_usage / credits_info.total_credits) * 100.0
        } else {
            0.0
        };
        
        let credits_text = vec![
            Line::from(vec![
                Span::styled("Total Credits: ", Style::default().fg(SUCCESS_COLOR).add_modifier(Modifier::BOLD)),
                Span::raw(format!("${:.2}", credits_info.total_credits)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Used Credits: ", Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD)),
                Span::raw(format!("${:.2}", credits_info.total_usage)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Remaining Credits: ", Style::default().fg(HIGHLIGHT_COLOR).add_modifier(Modifier::BOLD)),
                Span::raw(format!("${:.2}", remaining)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Usage Percentage: ", Style::default().fg(SECONDARY_COLOR).add_modifier(Modifier::BOLD)),
                Span::raw(format!("{:.1}%", percentage_used)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Last Updated: ", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
                Span::raw(format_system_time(credits_info.last_updated)),
            ]),
        ];
        
        let credits_paragraph = Paragraph::new(credits_text)
            .block(Block::default().borders(Borders::ALL).title("Credits"))
            .style(Style::default().fg(Color::White))
            .alignment(ratatui::layout::Alignment::Center);
        
        f.render_widget(credits_paragraph, chunks[1]);
    } else if app.processing_state == crate::app::ProcessingState::Processing {
        // Show loading message
        let loading_text = "Fetching credits information...";
        let loading_paragraph = Paragraph::new(loading_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(SECONDARY_COLOR))
            .alignment(ratatui::layout::Alignment::Center);
        
        f.render_widget(loading_paragraph, chunks[1]);
    } else {
        // Show error or no data message
        let error_text = match &app.processing_state {
            crate::app::ProcessingState::Error(err) => format!("Error: {}", err),
            _ => "No credits information available. Press 'r' to refresh.".to_string(),
        };
        
        let error_paragraph = Paragraph::new(error_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(ERROR_COLOR))
            .alignment(ratatui::layout::Alignment::Center);
        
        f.render_widget(error_paragraph, chunks[1]);
    }
    
    // Instructions
    let instructions = Paragraph::new("[R] Refresh credits | [Q] Return to previous screen")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(SECONDARY_COLOR));
    f.render_widget(instructions, chunks[2]);
}

/// Render the command bar
fn render_command_bar(f: &mut Frame, app: &App, area: Rect) {
    let command_text = match app.mode {
        AppMode::Welcome => "Enter: Continue | q: Quit",
        AppMode::Configuration => "Enter: Continue | Ctrl+A: Set API Key | q: Back",
        AppMode::ApiKeyInput => "Enter: Save API Key | Esc: Cancel",
        AppMode::CustomModelInput => "Enter: Save Custom Model | Esc: Cancel",
        AppMode::FileBrowser => "Enter: Open | Backspace/b: Up | s: Select | m: Multi-select | p: Prompt | h: Help | q: Back",
        AppMode::FileSelection => "Space: Toggle Selection | Enter: Continue | Esc/q: Cancel",
        AppMode::Editor => "p: Prompt | s: Select for Context | l: List Selected | q: Back",
        AppMode::PromptInput => "Enter: Submit | Esc: Cancel",
        AppMode::Results => {
            if app.multi_file_diffs.is_some() {
                "y: Apply All Changes | n: Discard | Left/Right: Switch Panels | Tab: Next File | e: Toggle Explanation | q: Back"
            } else {
                "y: Apply Changes | n: Discard | e: Toggle Explanation | q: Back"
            }
        },
        AppMode::Help => "q: Back",
        AppMode::Credits => "r: Refresh | q: Back",
    };
    
    let command_bar = Paragraph::new(command_text)
        .style(Style::default().fg(Color::White).bg(Color::Blue))
        .alignment(Alignment::Center);
    
    f.render_widget(command_bar, area);
}

/// Render the custom model input screen
pub fn render_custom_model_input(f: &mut Frame, app: &App, _area: Rect) {
    let size = f.size();
    
    // Create a block for the custom model input screen
    let block = Block::default()
        .title("Custom Model Input")
        .borders(Borders::ALL);
    
    // Create a layout for the input area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(3),  // Input field
            Constraint::Length(3),  // Instructions
        ])
        .split(size);
    
    // Render title
    let title = Paragraph::new("Enter your custom model identifier")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);
    
    // Render input field
    let input = Paragraph::new(app.current_prompt.as_str())
        .style(Style::default())
        .block(Block::default().borders(Borders::ALL).title("Custom Model"));
    f.render_widget(input, chunks[1]);
    
    // Set cursor position
    f.set_cursor(
        chunks[1].x + app.current_prompt.len() as u16 + 1,
        chunks[1].y + 1,
    );
    
    // Render instructions
    let instructions = Paragraph::new(
        "Enter a model identifier like 'google/gemini-2.0-flash-lite-001' | Press Enter to save | Esc to cancel"
    )
    .style(Style::default().fg(Color::Gray))
    .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[2]);
    
    // Render the main block
    f.render_widget(block, size);
} 