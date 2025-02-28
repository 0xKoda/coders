use anyhow::Result;
use crate::app::App;
use crate::ui;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};
use once_cell::sync::Lazy;

/// Channel for sending prompt results from async tasks to the main thread
pub static PROMPT_RESULT_TX: Lazy<Mutex<mpsc::Sender<PromptResult>>> = Lazy::new(|| {
    let (tx, _) = mpsc::channel();
    Mutex::new(tx)
});

/// Result of a prompt operation
#[derive(Debug)]
pub enum PromptResult {
    Success(crate::app::FileDiff),
    MultiFileSuccess(Vec<(String, crate::app::FileDiff)>),  // (filename, diff) pairs
    Error(String),
    CreditsInfo(crate::app::CreditsInfo),
}

/// Run the terminal UI
pub fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();
    
    // Initialize app (load config, etc)
    initialize_app(&mut app)?;

    // Create a channel for prompt results
    let (tx, rx) = mpsc::channel();
    
    // Store the sender in the static variable
    *PROMPT_RESULT_TX.lock().unwrap() = tx;

    // Run the event loop
    let res = run_event_loop(&mut terminal, &mut app, rx);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Handle any errors from the event loop
    if let Err(err) = res {
        println!("Error: {}", err);
    }

    Ok(())
}

/// Initialize the app state
fn initialize_app(app: &mut App) -> Result<()> {
    // Load saved API key if available
    if let Some(api_key) = load_api_key() {
        app.api_key = Some(api_key);
    }
    
    // Load configuration if available
    if let Some(model) = load_preferred_model() {
        app.selected_model = model;
    }
    
    // Initialize file list
    app.refresh_file_list()?;
    
    Ok(())
}

/// Run the main event loop
pub fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    rx: mpsc::Receiver<PromptResult>,
) -> Result<()> {
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();
    
    while app.running {
        // Show cursor for input modes, hide for others
        match app.mode {
            crate::app::AppMode::ApiKeyInput | crate::app::AppMode::PromptInput => {
                terminal.show_cursor()?;
            },
            _ => {
                terminal.hide_cursor()?;
            }
        }
        
        // Render the UI
        terminal.draw(|f| ui::render(f, app))?;
        
        // Check for prompt results
        if let Ok(result) = rx.try_recv() {
            // Handle prompt result
            match result {
                PromptResult::Success(ref _diff) => {
                    app.handle_prompt_result(result);
                }
                PromptResult::MultiFileSuccess(ref _diffs) => {
                    app.handle_prompt_result(result);
                }
                PromptResult::Error(ref _err) => {
                    app.handle_prompt_result(result);
                }
                PromptResult::CreditsInfo(ref _credits_info) => {
                    app.handle_prompt_result(result);
                }
            }
        }
        
        // Check for events with timeout
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
            
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // Only handle key press events (not releases)
                if key.kind == KeyEventKind::Press {
                    // Quit on Ctrl+C or Ctrl+D from any screen
                    if key.code == KeyCode::Char('c') && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) ||
                       key.code == KeyCode::Char('d') && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                        app.running = false;
                        continue;
                    }
                    
                    // Forward to the app's key handler
                    app.handle_key(key)?;
                }
            }
        }
        
        // Tick update for animations or periodic tasks
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
            
            // Update app state (spinner animation and message timeout)
            app.update();
        }
    }
    
    Ok(())
}

/// Load API key from configuration
fn load_api_key() -> Option<String> {
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

/// Load preferred model from configuration
fn load_preferred_model() -> Option<String> {
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