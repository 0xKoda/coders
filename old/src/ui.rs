use crate::app::{App, AppMode, ActivePanel, MessageType};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::path::Path;

// Color theme
const PRIMARY_COLOR: Color = Color::Rgb(65, 105, 225);      // Royal Blue
const SECONDARY_COLOR: Color = Color::Rgb(106, 90, 205);    // Slate Blue
const ACCENT_COLOR: Color = Color::Rgb(147, 112, 219);      // Medium Purple
const HIGHLIGHT_COLOR: Color = Color::Rgb(138, 43, 226);    // Blue Violet
const BG_COLOR: Color = Color::Rgb(25, 25, 40);            // Dark Blue-Grey
const SUCCESS_COLOR: Color = Color::Rgb(50, 205, 50);      // Lime Green
const ERROR_COLOR: Color = Color::Rgb(255, 69, 0);         // Red-Orange

/// Renders the UI based on the current application state
pub fn render(f: &mut Frame, app: &App) {
    // Determine which view to render based on app mode
    match app.mode {
        AppMode::Welcome => render_welcome(f, app),
        AppMode::Configuration => render_configuration(f, app),
        AppMode::ApiKeyInput => render_api_key_input(f, app),
        AppMode::FileBrowser => render_file_browser(f, app),
        AppMode::FileSelection => render_file_selection(f, app),
        AppMode::Editor => render_editor(f, app),
        AppMode::PromptInput => render_prompt_input(f, app),
        AppMode::Results => render_results(f, app),
        AppMode::Help => render_help(f, app),
        AppMode::Credits => render_credits(f, app),
    }
}

/// Renders the welcome screen
fn render_welcome(f: &mut Frame, _app: &App) {
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
fn render_configuration(f: &mut Frame, app: &App) {
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
fn render_api_key_input(f: &mut Frame, app: &App) {
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
fn render_file_browser(f: &mut Frame, app: &App) {
    let area = f.size();
    
    // Split the screen into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Title with current directory
            Constraint::Min(1),     // File list
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
            
            let name = if path.file_name().map_or(false, |name| name == "..") {
                "[Parent Directory]".to_string()
            } else {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "[Unknown]".to_string())
            };
            
            let display_name = if is_dir {
                format!("📁 {}/", name)
            } else {
                format!("📄 {}", name)
            };
            
            let style = if is_selected {
                Style::default().fg(HIGHLIGHT_COLOR).add_modifier(Modifier::BOLD)
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
    
    // Instructions
    let instructions = Paragraph::new("[↑/↓] Navigate | [Enter] Open | [Backspace] Go up | [H] Help | [Q] Quit")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(SECONDARY_COLOR));
    f.render_widget(instructions, chunks[2]);
}

/// Renders the file selection screen
fn render_file_selection(f: &mut Frame, app: &App) {
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
fn render_editor(f: &mut Frame, app: &App) {
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
        render_credits_info(f, app, main_chunks[3]);
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
    
    // Convert Option<String> to String for display
    let message_text = app.message.clone().unwrap_or_default();
    
    let message_bar = Paragraph::new(message_text)
        .style(message_style);
    
    f.render_widget(message_bar, area);
}

/// Render credits information
fn render_credits_info(f: &mut Frame, app: &App, area: Rect) {
    let credits_info = &app.credits_info;
    
    // Format the last updated time
    let last_updated = credits_info.last_updated
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let time_diff = now - last_updated;
    let time_str = if time_diff < 60 {
        format!("{} seconds ago", time_diff)
    } else if time_diff < 3600 {
        format!("{} minutes ago", time_diff / 60)
    } else if time_diff < 86400 {
        format!("{} hours ago", time_diff / 3600)
    } else {
        format!("{} days ago", time_diff / 86400)
    };
    
    let credits_text = format!(
        "OpenRouter Credits: ${:.4} (Used: ${:.4}) - Last Updated: {}",
        credits_info.total_credits,
        credits_info.total_usage,
        time_str
    );
    
    let credits_paragraph = Paragraph::new(credits_text)
        .block(Block::default().borders(Borders::ALL).title("Credits Info"))
        .style(Style::default().fg(ACCENT_COLOR));
    
    f.render_widget(credits_paragraph, area);
}

/// Renders the prompt input screen
fn render_prompt_input(f: &mut Frame, app: &App) {
    let area = f.size();
    
    // Split the screen into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(5),     // Code preview
            Constraint::Length(5),  // Prompt input
            Constraint::Length(3),  // Instructions
        ].as_ref())
        .split(area);
    
    // Title
    let file_name = app.current_file
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "[No File]".to_string());
    
    let title = Paragraph::new(format!("Editing: {}", file_name))
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(PRIMARY_COLOR));
    f.render_widget(title, chunks[0]);
    
    // Code preview (abbreviated)
    let preview_lines = app.current_file_content
        .lines()
        .take(10)  // Show first 10 lines
        .enumerate()
        .map(|(i, line)| {
            Line::from(vec![
                Span::styled(
                    format!("{:4} ", i + 1),
                    Style::default().fg(SECONDARY_COLOR)
                ),
                Span::raw(line),
            ])
        })
        .collect::<Vec<_>>();
    
    let preview = Paragraph::new(preview_lines)
        .block(Block::default().borders(Borders::ALL).title("Code Preview"))
        .style(Style::default().fg(Color::White));
    
    f.render_widget(preview, chunks[1]);
    
    // Prompt input or processing indicator
    match app.processing_state {
        crate::app::ProcessingState::Processing => {
            // Show a spinner and "Cooking..." message
            let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let spinner = spinner_chars[app.spinner_frame % spinner_chars.len()];
            
            let processing_text = format!("{} Cooking up some code magic...", spinner);
            let processing_paragraph = Paragraph::new(processing_text)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Processing")
                    .border_style(Style::default().fg(ACCENT_COLOR)))
                .style(Style::default().fg(HIGHLIGHT_COLOR))
                .alignment(ratatui::layout::Alignment::Center);
            
            f.render_widget(processing_paragraph, chunks[2]);
        },
        crate::app::ProcessingState::Error(ref error) => {
            // Show error message
            let error_text = format!("Error: {}", error);
            let error_paragraph = Paragraph::new(error_text)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Error")
                    .border_style(Style::default().fg(ERROR_COLOR)))
                .style(Style::default().fg(ERROR_COLOR))
                .alignment(ratatui::layout::Alignment::Center);
            
            f.render_widget(error_paragraph, chunks[2]);
        },
        _ => {
            // Normal prompt input
            let prompt_block = Block::default()
                .borders(Borders::ALL)
                .title("Enter your prompt:")
                .border_style(Style::default().fg(HIGHLIGHT_COLOR));
            
            let prompt_text = Paragraph::new(app.current_prompt.as_str())
                .block(prompt_block)
                .style(Style::default().fg(Color::White))
                .wrap(Wrap { trim: true });
            
            f.render_widget(prompt_text, chunks[2]);
            
            // Calculate cursor position correctly
            // The cursor should be positioned at the start of the input area + the length of the prompt
            // We need to account for the border and padding
            let cursor_x = chunks[2].x + 1 + app.current_prompt.len() as u16;
            let cursor_y = chunks[2].y + 1; // Position at the first line inside the block
            
            // Set cursor position
            f.set_cursor(cursor_x, cursor_y);
        }
    }
    
    // Instructions
    let instructions_text = match app.processing_state {
        crate::app::ProcessingState::Processing => 
            "Please wait while your request is being processed...",
        crate::app::ProcessingState::Error(_) => 
            "[Esc] Return to editor | [Enter] Try again",
        _ => 
            "[Enter] Submit prompt | [Esc] Cancel | [Tab] Complete prompt",
    };
    
    let instructions = Paragraph::new(instructions_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(SECONDARY_COLOR));
    f.render_widget(instructions, chunks[3]);
}

/// Renders the results screen with diff
fn render_results(f: &mut Frame, app: &App) {
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
    
    // Title
    let title = Paragraph::new("Review Changes")
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
    
    // Instructions
    let instructions = Paragraph::new("[Y] Apply changes | [N] Discard | [Tab] Switch panels | [↑/↓] Scroll")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(SECONDARY_COLOR));
    f.render_widget(instructions, main_chunks[2]);
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
fn render_help(f: &mut Frame, _app: &App) {
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

/// Renders the credits screen
fn render_credits(f: &mut Frame, app: &App) {
    let area = f.size();
    
    // Split the screen into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(10), // Credits info
            Constraint::Min(0),     // Spacer
            Constraint::Length(3),  // Instructions
        ].as_ref())
        .split(area);
    
    // Title
    let title = Paragraph::new("OpenRouter Credits")
        .style(Style::default().fg(PRIMARY_COLOR).add_modifier(Modifier::BOLD))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(title, chunks[0]);
    
    // Credits information
    let credits_info = &app.credits_info;
    
    // Format the last updated time
    let last_updated = credits_info.last_updated
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let time_diff = now - last_updated;
    let time_str = if time_diff < 60 {
        format!("{} seconds ago", time_diff)
    } else if time_diff < 3600 {
        format!("{} minutes ago", time_diff / 60)
    } else if time_diff < 86400 {
        format!("{} hours ago", time_diff / 3600)
    } else {
        format!("{} days ago", time_diff / 86400)
    };
    
    let credits_text = vec![
        Line::from(vec![
            Span::styled("Total Credits: ", Style::default().fg(SECONDARY_COLOR)),
            Span::styled(format!("${:.4}", credits_info.total_credits), Style::default().fg(HIGHLIGHT_COLOR)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Total Usage: ", Style::default().fg(SECONDARY_COLOR)),
            Span::styled(format!("${:.4}", credits_info.total_usage), Style::default().fg(ACCENT_COLOR)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Last Updated: ", Style::default().fg(SECONDARY_COLOR)),
            Span::styled(time_str, Style::default().fg(Color::White)),
        ]),
    ];
    
    let credits_paragraph = Paragraph::new(credits_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default())
        .alignment(ratatui::layout::Alignment::Left);
    
    f.render_widget(credits_paragraph, chunks[1]);
    
    // Instructions
    let instructions = Paragraph::new("[R] Refresh | [Q] Back")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(SECONDARY_COLOR));
    f.render_widget(instructions, chunks[3]);
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