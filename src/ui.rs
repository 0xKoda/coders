use crate::app::{App, AppMode, ActivePanel, MessageType};
use crate::tui::PromptResult;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Utc};

// Color theme
const PRIMARY_COLOR: Color = Color::Rgb(65, 105, 225);      // Royal Blue
const SECONDARY_COLOR: Color = Color::Rgb(106, 90, 205);    // Slate Blue
const ACCENT_COLOR: Color = Color::Rgb(147, 112, 219);      // Medium Purple
const HIGHLIGHT_COLOR: Color = Color::Rgb(138, 43, 226);    // Blue Violet
const BG_COLOR: Color = Color::Rgb(25, 25, 40);            // Dark Blue-Grey
const SUCCESS_COLOR: Color = Color::Rgb(50, 205, 50);      // Lime Green
const ERROR_COLOR: Color = Color::Rgb(255, 69, 0);         // Red-Orange

/// Render the UI
pub fn render(f: &mut Frame, app: &App) {
    let area = f.size();
    
    // Split the screen into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Status bar
            Constraint::Min(1),     // Main content
            Constraint::Length(3),  // Message bar
        ].as_ref())
        .split(area);
    
    // Render status bar
    render_status_bar(f, app, chunks[0]);
    
    // Render main content based on current mode
    match app.mode {
        AppMode::Welcome => render_welcome(f, app, chunks[1]),
        AppMode::Configuration => render_configuration(f, app, chunks[1]),
        AppMode::ApiKeyInput => render_api_key_input(f, app, chunks[1]),
        AppMode::FileBrowser => render_file_browser(f, app, chunks[1]),
        AppMode::FileSelection => render_file_selection(f, app, chunks[1]),
        AppMode::Editor => render_editor(f, app, chunks[1]),
        AppMode::PromptInput => render_prompt_input(f, app, chunks[1]),
        AppMode::Results => render_results(f, app, chunks[1]),
        AppMode::Help => render_help(f, app, chunks[1]),
        AppMode::Credits => render_credits_screen(f, app, chunks[1]),
    }
    
    // Render message bar
    render_message_bar(f, app, chunks[2]);
    
    // Show credits popup if enabled (in any mode)
    if app.show_credits {
        render_credits(f, app, area);
    }
    
    // Show spinner overlay only if processing and not in prompt input mode
    // This prevents duplicate spinners when in prompt input mode
    if app.processing_state == crate::app::ProcessingState::Processing && app.mode != AppMode::PromptInput {
        render_spinner(f, app, area);
    }
}

/// Renders the welcome screen
fn render_welcome(f: &mut Frame, _app: &App, area: Rect) {
    let area = f.size();
    
    // Create a centered area for the welcome message
    let welcome_area = centered_rect(60, 20, area);
    
    // Create the welcome message
    let welcome_text = vec![
        Span::styled(
            "Code-AI Assistant",
            Style::default()
                .fg(HIGHLIGHT_COLOR)
                .add_modifier(Modifier::BOLD)
        ),
        Span::raw("\n\n"),
        Span::styled(
            "An AI-powered code editing assistant",
            Style::default().fg(SECONDARY_COLOR)
        ),
        Span::raw("\n\n"),
        Span::raw("Press "),
        Span::styled("ENTER", Style::default().fg(ACCENT_COLOR)),
        Span::raw(" to start"),
        Span::raw("\nPress "),
        Span::styled("q", Style::default().fg(ACCENT_COLOR)),
        Span::raw(" to quit"),
    ];
    
    // Convert Vec<Span> to Text using Line
    let text = Text::from(Line::from(welcome_text));
    
    let welcome_paragraph = Paragraph::new(text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(PRIMARY_COLOR))
            .title("Welcome")
            .title_style(Style::default().fg(PRIMARY_COLOR).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(BG_COLOR))
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
    
    f.render_widget(welcome_paragraph, welcome_area);
}

/// Renders the configuration screen
fn render_configuration(f: &mut Frame, app: &App, area: Rect) {
    let area = f.size();
    
    // Split the screen into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(3),  // API Key info
            Constraint::Length(6),  // Model selection
            Constraint::Min(0),     // Spacer
            Constraint::Length(3),  // Instructions
        ].as_ref())
        .split(area);
    
    // Title
    let title = Paragraph::new("Configuration")
        .style(Style::default().fg(PRIMARY_COLOR).add_modifier(Modifier::BOLD))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(title, chunks[0]);
    
    // API Key section
    let api_status = match &app.api_key {
        Some(_) => "API Key: ✓ Configured",
        None => "API Key: ✗ Not Configured (Press Ctrl+A to add)",
    };
    
    let api_key_text = Paragraph::new(api_status)
        .block(Block::default().borders(Borders::ALL).title("OpenRouter API"))
        .style(Style::default().fg(if app.api_key.is_some() { SUCCESS_COLOR } else { ERROR_COLOR }));
    f.render_widget(api_key_text, chunks[1]);
    
    // Model selection
    let models: Vec<ListItem> = app.available_models
        .iter()
        .enumerate()
        .map(|(i, model)| {
            let selected = model == &app.selected_model;
            let style = if selected {
                Style::default().fg(HIGHLIGHT_COLOR).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            
            ListItem::new(Line::from(Span::styled(
                format!("{}. {}", i + 1, model),
                style,
            )))
        })
        .collect();
    
    let models_list = List::new(models)
        .block(Block::default().borders(Borders::ALL).title("Available Models"))
        .highlight_style(Style::default().fg(HIGHLIGHT_COLOR).add_modifier(Modifier::BOLD));
    f.render_widget(models_list, chunks[2]);
    
    // Instructions
    let instructions = Paragraph::new("Press [Ctrl+A] to add API key | [Enter] to confirm | [↑/↓] to select model | [Q] to go back")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(SECONDARY_COLOR));
    f.render_widget(instructions, chunks[4]);
}

/// Renders the API key input screen
fn render_api_key_input(f: &mut Frame, app: &App, area: Rect) {
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
fn render_file_browser(f: &mut Frame, app: &App, area: Rect) {
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
fn render_file_selection(f: &mut Frame, app: &App, area: Rect) {
    let area = f.size();
    
    // Split the screen into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Title with current directory
            Constraint::Min(1),     // File list
            Constraint::Length(5),  // Selected files
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
        .block(Block::default().borders(Borders::ALL).title("Files (Space to select)"))
        .highlight_style(Style::default().fg(HIGHLIGHT_COLOR).add_modifier(Modifier::BOLD));
    
    f.render_widget(files_list, chunks[1]);
    
    // Selected files
    let selected_files_text = if app.selected_files.is_empty() {
        vec![Line::from(Span::styled(
            "No files selected. Use spacebar to select files.",
            Style::default().fg(ERROR_COLOR)
        ))]
    } else {
        let mut lines = vec![Line::from(Span::styled(
            format!("{} files selected:", app.selected_files.len()),
            Style::default().fg(SUCCESS_COLOR).add_modifier(Modifier::BOLD)
        ))];
        
        for path in &app.selected_files {
            if let Some(file_name) = path.file_name() {
                lines.push(Line::from(Span::styled(
                    format!("- {}", file_name.to_string_lossy()),
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
    let instructions = Paragraph::new("[Space] Toggle selection | [Enter] Proceed to prompt | [Q] Cancel | [↑/↓] Navigate")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(SECONDARY_COLOR));
    f.render_widget(instructions, chunks[3]);
}

/// Renders the code editor
fn render_editor(f: &mut Frame, app: &App, area: Rect) {
    let area = f.size();
    
    // Split the screen into sections
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // File info
            Constraint::Min(1),     // Code viewer and sidebar
            Constraint::Length(3),  // Status bar
            Constraint::Length(if app.show_credits { 3 } else { 1 }),  // Message bar or credits info
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
        .block(Block::default().borders(Borders::ALL).title("Code"))
        .style(Style::default().fg(Color::White))
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
    
    // Message bar or credits info
    if app.show_credits {
        render_credits(f, app, main_chunks[3]);
    } else {
        render_message_bar(f, app, main_chunks[3]);
    }
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
fn render_message_bar(f: &mut Frame, app: &App, area: Rect) {
    let message_style = match app.message_type {
        MessageType::Info => Style::default().fg(SECONDARY_COLOR),
        MessageType::Error => Style::default().fg(ERROR_COLOR),
        MessageType::Success => Style::default().fg(SUCCESS_COLOR),
    };
    
    let message_text = app.message.clone().unwrap_or_default();
    let message_bar = Paragraph::new(message_text)
        .style(message_style);
    
    f.render_widget(message_bar, area);
}

/// Render credits information
fn render_credits(f: &mut Frame, app: &App, area: Rect) {
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

/// Renders the prompt input screen
fn render_prompt_input(f: &mut Frame, app: &App, area: Rect) {
    let area = f.size();
    
    // Split the screen into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(10),    // Prompt input area
            Constraint::Length(3),  // Instructions
        ].as_ref())
        .split(area);
    
    // Title
    let title_text = if !app.selected_files.is_empty() {
        format!("Enter Prompt (Using {} files as context)", app.selected_files.len())
    } else if let Some(file) = &app.current_file {
        format!("Enter Prompt for {}", file.file_name().unwrap_or_default().to_string_lossy())
    } else {
        "Enter Prompt".to_string()
    };
    
    let title = Paragraph::new(title_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(PRIMARY_COLOR));
    f.render_widget(title, chunks[0]);
    
    // Prompt input area
    let prompt_block = Block::default()
        .borders(Borders::ALL)
        .title("Prompt")
        .style(Style::default());
    
    // If processing, show a cooking animation in the prompt area
    if app.processing_state == crate::app::ProcessingState::Processing {
        let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let frame_idx = app.spinner_frame % spinner_frames.len();
        
        let cooking_messages = [
            "Cooking up some code...",
            "Stirring the algorithm pot...",
            "Adding a pinch of logic...",
            "Simmering the solution...",
            "Whisking some functions...",
            "Baking the perfect code...",
            "Sprinkling syntax sugar...",
            "Marinating the modules...",
            "Sautéing the subroutines...",
            "Preparing a delicious patch...",
        ];
        let message_idx = (app.spinner_frame / 5) % cooking_messages.len();
        
        let spinner_text = format!("{} {}", spinner_frames[frame_idx], cooking_messages[message_idx]);
        
        let spinner_paragraph = Paragraph::new(spinner_text)
            .block(prompt_block)
            .style(Style::default().fg(ACCENT_COLOR))
            .alignment(ratatui::layout::Alignment::Center);
        
        f.render_widget(spinner_paragraph, chunks[1]);
    } else {
        // Normal prompt input display
        let prompt_paragraph = Paragraph::new(app.current_prompt.as_str())
            .block(prompt_block)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        
        f.render_widget(prompt_paragraph, chunks[1]);
    }
    
    // Instructions
    let instructions = if app.processing_state == crate::app::ProcessingState::Processing {
        "Processing your request... Please wait."
    } else {
        "[Enter] Submit | [Esc/q] Cancel"
    };
    
    let instructions_paragraph = Paragraph::new(instructions)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(SECONDARY_COLOR));
    
    f.render_widget(instructions_paragraph, chunks[2]);
    
    // If there are selected files, show them in a sidebar
    if !app.selected_files.is_empty() {
        let sidebar_width = 30.min(area.width / 3);
        
        let sidebar_area = Rect {
            x: area.width - sidebar_width - 1,
            y: 1,
            width: sidebar_width,
            height: area.height - 2,
        };
        
        render_sidebar(f, app, sidebar_area);
    }
}

/// Renders the results screen with diff
fn render_results(f: &mut Frame, app: &App, area: Rect) {
    let area = f.size();
    
    // Split the screen into sections
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(1),     // Diff panels
            Constraint::Length(3),  // Instructions
        ].as_ref())
        .margin(1)
        .split(area);
    
    // Title with file information if available
    let title_text = if let Some(file_name) = &app.current_diff_file {
        // For multi-file diffs, show which file we're viewing
        if let Some(diffs) = &app.multi_file_diffs {
            if diffs.len() > 1 {
                if let Some(idx) = diffs.iter().position(|(name, _)| name == file_name) {
                    format!("Review Changes - File {} of {}: {}", idx + 1, diffs.len(), file_name)
                } else {
                    format!("Review Changes - {}", file_name)
                }
            } else {
                format!("Review Changes - {}", file_name)
            }
        } else {
            format!("Review Changes - {}", file_name)
        }
    } else {
        "Review Changes".to_string()
    };
    
    let title = Paragraph::new(title_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(PRIMARY_COLOR));
    f.render_widget(title, main_chunks[0]);
    
    // Split the main area into two panels
    let diff_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ].as_ref())
        .split(main_chunks[1]);
    
    if let Some(diff) = &app.current_diff {
        // Original code panel
        render_original_panel(f, app, diff, diff_chunks[0]);
        
        // Modified code panel
        render_modified_panel(f, app, diff, diff_chunks[1]);
    } else {
        // If no diff is available, show a message
        let no_diff_text = Paragraph::new("No changes to display")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Red))
            .alignment(ratatui::layout::Alignment::Center);
        
        f.render_widget(no_diff_text, main_chunks[1]);
    }
    
    // Instructions - add navigation instructions for multi-file diffs
    let instructions = if let Some(diffs) = &app.multi_file_diffs {
        if diffs.len() > 1 {
            "[Y] Apply all changes | [N] Discard | [←/→] Navigate files | [Tab] Switch panels | [↑/↓] Scroll"
        } else {
            "[Y] Apply changes | [N] Discard | [Tab] Switch panels | [↑/↓] Scroll"
        }
    } else {
        "[Y] Apply changes | [N] Discard | [Tab] Switch panels | [↑/↓] Scroll"
    };
    
    let instructions_paragraph = Paragraph::new(instructions)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(SECONDARY_COLOR));
    f.render_widget(instructions_paragraph, main_chunks[2]);
}

/// Renders the original code panel
fn render_original_panel(
    f: &mut Frame,
    app: &App,
    diff: &crate::app::FileDiff,
    area: Rect,
) {
    // Split the area into header and body
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(area);

    // Create header
    let header = Paragraph::new("Original Code")
        .style(Style::default().fg(PRIMARY_COLOR))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // Create styled text for original code
    let mut text = Text::default();
    let original_lines: Vec<&str> = diff.original.lines().collect();
    
    for (i, line) in original_lines.iter().enumerate() {
        // Check if this line is affected by a change
        let style = if diff.changes.iter().any(|change| change.line_number == i + 1) {
            Style::default().fg(ERROR_COLOR)
        } else {
            Style::default()
        };
        
        text.lines.push(Line::from(vec![
            Span::styled(
                format!("{:4} ", i + 1),
                Style::default().fg(SECONDARY_COLOR)
            ),
            Span::styled(line.to_string(), style),
        ]));
    }

    // Create the code paragraph with scrolling
    let code_paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .scroll((app.scroll_position as u16, 0))
        .style(
            if app.active_panel == ActivePanel::Left {
                Style::default().bg(BG_COLOR)
            } else {
                Style::default()
            }
        );
    
    f.render_widget(code_paragraph, chunks[1]);
}

/// Renders the modified code panel
fn render_modified_panel(
    f: &mut Frame,
    app: &App,
    diff: &crate::app::FileDiff,
    area: Rect,
) {
    // Split the area into header and body
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(area);

    // Create header
    let header = Paragraph::new("Modified Code")
        .style(Style::default().fg(PRIMARY_COLOR))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // Create styled text for modified code with highlighted changes
    let mut text = Text::default();
    let modified_lines: Vec<&str> = diff.modified.lines().collect();
    
    for (i, line) in modified_lines.iter().enumerate() {
        // Determine the style based on change type
        let style = diff.changes.iter()
            .find(|change| change.line_number == i + 1)
            .map(|change| match change.change_type {
                crate::app::ChangeType::Insert => Style::default().fg(SUCCESS_COLOR),
                crate::app::ChangeType::Delete => Style::default().fg(ERROR_COLOR),
                crate::app::ChangeType::Modify => Style::default().fg(ACCENT_COLOR),
            })
            .unwrap_or_else(|| Style::default());
        
        text.lines.push(Line::from(vec![
            Span::styled(
                format!("{:4} ", i + 1),
                Style::default().fg(SECONDARY_COLOR)
            ),
            Span::styled(line.to_string(), style),
        ]));
    }

    // Create the code paragraph with scrolling
    let code_paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .scroll((app.scroll_position as u16, 0))
        .style(
            if app.active_panel == ActivePanel::Right {
                Style::default().bg(BG_COLOR)
            } else {
                Style::default()
            }
        );
    
    f.render_widget(code_paragraph, chunks[1]);
}

/// Renders the help screen
fn render_help(f: &mut Frame, _app: &App, area: Rect) {
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
        Line::from("  [Tab] - Switch between original and modified code"),
        Line::from("  [↑/↓] - Scroll through code"),
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
    
    let spinner_text = format!("{} Processing request...", spinner);
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