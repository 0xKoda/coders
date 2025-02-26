use anyhow::{Context, Result};
use clap::Parser;
use reqwest::Client;
use serde_json::json;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use indicatif::{ProgressBar, ProgressStyle};
use colored::*;
use std::time::Duration;
use std::path::Path;
// Add Ratatui imports
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
    Terminal,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

// Default models for OpenRouter
const DEFAULT_CLAUDE: &str = "anthropic/claude-3.7-sonnet:beta";
const DEFAULT_GEMINI: &str = "google/gemini-2.0-flash-001";
const AST_GENERATION_MODEL: &str = "google/gemini-2.0-flash-001"; // Use Gemini for AST generation

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, required_unless_present = "reset")]
    file: Option<String>,
    
    #[arg(short, long, help = "Select a model interactively")]
    model: bool,
    
    #[arg(short = 'u', long, help = "Use custom model (provide model name)")]
    custom_model: Option<String>,
    
    #[arg(
        short = 'c', 
        long, 
        help = "Use Claude 3.7 Sonnet (anthropic/claude-3.7-sonnet:beta)"
    )]
    claude: bool,
    
    #[arg(
        short = 'g', 
        long, 
        help = "Use Gemini 2.0 Flash (google/gemini-2.0-flash-001)"
    )]
    gemini: bool,
    
    #[arg(short, long, help = "Reset API key")]
    reset: bool,
    
    #[arg(short, long, help = "Enable verbose output")]
    verbose: bool,
    
    #[arg(short = 'n', long, help = "Disable AST generation (AST is enabled by default)")]
    no_ast: bool,
}

#[derive(Debug)]
enum ChangeType {
    Insert,
    Delete,
    Modify,
}

#[derive(Debug)]
struct Change {
    change_type: ChangeType,
    line_number: usize,
    content: String,
}

// Define TUI app state
struct TuiApp<'a> {
    original_code: Vec<&'a str>,
    modified_code: Vec<&'a str>,
    changes: Vec<Change>,
    user_prompt: &'a str,
    file_path: &'a str,
    scroll_position: u16,
    selected_side: Side,
}

#[derive(PartialEq)]
enum Side {
    Left,
    Right,
}

impl<'a> TuiApp<'a> {
    fn new(original_code: &'a str, modified_code: &'a str, changes: Vec<Change>, user_prompt: &'a str, file_path: &'a str) -> Self {
        Self {
            original_code: original_code.lines().collect(),
            modified_code: modified_code.lines().collect(),
            changes,
            user_prompt,
            file_path,
            scroll_position: 0,
            selected_side: Side::Left,
        }
    }

    fn scroll_up(&mut self) {
        if self.scroll_position > 0 {
            self.scroll_position -= 1;
        }
    }

    fn scroll_down(&mut self) {
        let max_lines = self.original_code.len().max(self.modified_code.len());
        if self.scroll_position < max_lines as u16 {
            self.scroll_position += 1;
        }
    }

    fn toggle_side(&mut self) {
        self.selected_side = match self.selected_side {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        };
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    println!("Starting code-ai application");

    if cli.reset {
        reset_api_key("OpenRouter")?;
        return Ok(());
    }

    // Since we're not resetting, file must be present at this point
    let file_path = cli.file.as_deref().unwrap();

    let api_key = get_or_prompt_for_api_key("OpenRouter").await?;

    // Read the input file
    if cli.verbose {
        println!("Reading file: {}", file_path);
    }
    let file_content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path))?;

    // Generate AST if not disabled
    let ast = if !cli.no_ast {
        let spinner = display_waiting_message("Generating AST for your code...");
        let language = get_file_language(file_path);
        let ast_result = generate_ast(&api_key, &file_content, language).await;
        spinner.finish_and_clear();
        
        match ast_result {
            Ok(ast) => {
                if cli.verbose {
                    println!("AST generated successfully");
                }
                Some(ast)
            }
            Err(e) => {
                eprintln!("Failed to generate AST: {}", e);
                println!("Continuing without AST...");
                None
            }
        }
    } else {
        if cli.verbose {
            println!("AST generation disabled");
        }
        None
    };

    // Get user prompt
    let prompt = prompt_for_user_input()?;
    
    // Create context with or without AST
    let context = match ast {
        Some(ast_content) => {
            format!(
                "{}\n\nCode:\n{}\n\nAbstract Syntax Tree:\n{}", 
                prompt, file_content, ast_content
            )
        }
        None => format!("{}\n\n{}", prompt, file_content),
    };

    // Determine which model to use
    let model = determine_model(&cli)?;
    println!("Using model: {}", model);

    // Send request to OpenRouter
    let response = send_request_to_openrouter(&api_key, &context, &model, file_path, cli.verbose).await?;

    match response {
        Some(content) => {
            println!("API Response received. Opening diff viewer...");
            // Pass the user prompt to the TUI diff viewer
            show_diff_and_prompt_for_changes(&file_content, &content, file_path, &prompt)?;
        }
        None => {
            eprintln!("No valid response received from the API.");
            println!("No valid response received from the API.");
        }
    }

    Ok(())
}

async fn generate_ast(api_key: &str, code: &str, language: &str) -> Result<String> {
    let client = Client::new();
    let url = "https://openrouter.ai/api/v1/chat/completions";

    let request_body = json!({
        "model": AST_GENERATION_MODEL,
        "messages": [
            {
                "role": "system", 
                "content": "You are an expert programmer specializing in code analysis. Generate a concise Abstract Syntax Tree (AST) for the provided code. Focus on the key structural elements without being overly verbose. The AST should be clear, accurate, and helpful for understanding the code structure."
            },
            {
                "role": "user", 
                "content": "Generate an AST for this JavaScript code: function add(a, b) { return a + b; }"
            },
            {
                "role": "assistant", 
                "content": "Program\n  └─ FunctionDeclaration (name: add)\n     ├─ Parameters\n     │  ├─ Identifier (name: a)\n     │  └─ Identifier (name: b)\n     └─ BlockStatement\n        └─ ReturnStatement\n           └─ BinaryExpression (operator: +)\n              ├─ Identifier (name: a)\n              └─ Identifier (name: b)"
            },
            {
                "role": "user", 
                "content": "Generate an AST for this Python code: def factorial(n): if n <= 1: return 1 else: return n * factorial(n-1)"
            },
            {
                "role": "assistant", 
                "content": "Module\n  └─ FunctionDef (name: factorial)\n     ├─ Parameters\n     │  └─ Parameter (name: n)\n     └─ Body\n        └─ If\n           ├─ Test: Compare\n           │  ├─ Left: Name (id: n)\n           │  └─ Comparator: LessThanOrEqual (<=)\n           │     └─ Constant (value: 1)\n           ├─ Body\n           │  └─ Return\n           │     └─ Constant (value: 1)\n           └─ Orelse\n              └─ Return\n                 └─ BinOp (op: Mult)\n                    ├─ Left: Name (id: n)\n                    └─ Right: Call\n                       ├─ Func: Name (id: factorial)\n                       └─ Args: BinOp (op: Sub)\n                          ├─ Left: Name (id: n)\n                          └─ Right: Constant (value: 1)"
            },
            {
                "role": "user", 
                "content": "Generate an AST for this Rust code: fn main() { println!(\"Hello, world!\"); }"
            },
            {
                "role": "assistant", 
                "content": "Crate\n  └─ Function (name: main)\n     ├─ Parameters: []\n     └─ Body: Block\n        └─ MacroCall (name: println)\n           └─ Arguments\n              └─ Literal (type: string, value: \"Hello, world!\")"
            },
            {
                "role": "user", 
                "content": "Generate an AST for this JavaScript code: function divide(a, b) { if (b === 0) { throw new Error(\"Division by zero\"); } return a / b; }"
            },
            {
                "role": "assistant", 
                "content": "Program\n  └─ FunctionDeclaration (name: divide)\n     ├─ Parameters\n     │  ├─ Identifier (name: a)\n     │  └─ Identifier (name: b)\n     └─ BlockStatement\n        ├─ IfStatement\n        │  ├─ Test: BinaryExpression (operator: ===)\n        │  │  ├─ Left: Identifier (name: b)\n        │  │  └─ Right: Literal (value: 0)\n        │  └─ Consequent: BlockStatement\n        │     └─ ThrowStatement\n        │        └─ NewExpression\n        │           ├─ Callee: Identifier (name: Error)\n        │           └─ Arguments\n        │              └─ Literal (value: \"Division by zero\")\n        └─ ReturnStatement\n           └─ BinaryExpression (operator: /)\n              ├─ Left: Identifier (name: a)\n              └─ Right: Identifier (name: b)"
            },
            {
                "role": "user", 
                "content": format!("Generate an Abstract Syntax Tree (AST) for the following {} code:\n\n{}", language, code)
            }
        ],
        "max_tokens": 1500,
        "temperature": 0.7,
    });

    let response = client.post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    if response.status().is_success() {
        let body = response.text().await?;
        let json_response: serde_json::Value = serde_json::from_str(&body)?;
        
        if let Some(content) = json_response["choices"][0]["message"]["content"].as_str() {
            Ok(content.to_string())
        } else {
            Err(anyhow::anyhow!("Failed to extract AST from response"))
        }
    } else {
        Err(anyhow::anyhow!("API request failed: {}", response.status()))
    }
}

fn determine_model(cli: &Cli) -> Result<String> {
    if let Some(custom_model) = &cli.custom_model {
        // User provided a custom model name
        if cli.verbose {
            println!("Using custom model: {}", custom_model);
        }
        return Ok(custom_model.clone());
    }
    
    if cli.claude {
        // Use Claude model
        return Ok(DEFAULT_CLAUDE.to_string());
    }
    
    if cli.gemini {
        // Use Gemini model
        return Ok(DEFAULT_GEMINI.to_string());
    }
    
    if cli.model {
        // Interactive model selection
        return select_model_interactive();
    }
    
    // Default to Claude if no option specified
    if cli.verbose {
        println!("No model specified, defaulting to Claude 3.7 Sonnet");
    }
    Ok(DEFAULT_CLAUDE.to_string())
}

fn select_model_interactive() -> Result<String> {
    println!("Select a model:");
    println!("1. {} (Claude 3.7 Sonnet)", DEFAULT_CLAUDE);
    println!("2. {} (Gemini 2.0 Flash)", DEFAULT_GEMINI);
    println!("3. Enter custom model name");

    loop {
        print!("Enter the number of your choice: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "1" => return Ok(DEFAULT_CLAUDE.to_string()),
            "2" => return Ok(DEFAULT_GEMINI.to_string()),
            "3" => {
                print!("Enter the custom model name: ");
                io::stdout().flush()?;
                let mut model_name = String::new();
                io::stdin().read_line(&mut model_name)?;
                return Ok(model_name.trim().to_string());
            }
            _ => println!("Invalid choice. Please try again."),
        }
    }
}

fn reset_api_key(provider: &str) -> Result<()> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?;
    let config_file = config_dir.join(format!("{}_api_key.txt", provider.to_lowercase()));

    if config_file.exists() {
        std::fs::remove_file(&config_file)?;
        println!("{} API key has been reset. You will be prompted for a new key on the next run.", provider);
    } else {
        println!("No existing {} API key found. You will be prompted for a key on the next run.", provider);
    }
    
    Ok(())
}

async fn get_or_prompt_for_api_key(api_name: &str) -> Result<String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?;
    let config_file = config_dir.join(format!("{}_api_key.txt", api_name.to_lowercase()));

    let api_key = if config_file.exists() {
        let api_key = fs::read_to_string(&config_file)?;
        if api_key.trim().is_empty() {
            prompt_and_save_api_key(api_name, &config_file)?
        } else {
            api_key.trim().to_string()
        }
    } else {
        prompt_and_save_api_key(api_name, &config_file)?
    };

    if validate_api_key(api_name, &api_key).await? {
        fs::write(&config_file, &api_key)?;
        println!("{} API key validated and saved successfully", api_name);
        Ok(api_key)
    } else {
        println!("Invalid {} API key. Please enter a valid key.", api_name);
        let new_api_key = prompt_and_save_api_key(api_name, &config_file)?;
        println!("New {} API key saved successfully", api_name);
        Ok(new_api_key)
    }
}

async fn validate_api_key(api_name: &str, api_key: &str) -> Result<bool> {
    let client = Client::new();
    let url = match api_name {
        "OpenRouter" => "https://openrouter.ai/api/v1/models",
        _ => return Err(anyhow::anyhow!("Unknown API provider")),
    };

    let response = client.get(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let is_valid = response.status().is_success();
    if !is_valid && response.status().as_u16() != 404 {
        eprintln!("API key validation failed with status: {}", response.status());
    }
    
    Ok(is_valid)
}

fn prompt_and_save_api_key(api_name: &str, config_file: &PathBuf) -> Result<String> {
    let api_key = prompt_for_api_key(api_name)?;
    fs::create_dir_all(config_file.parent().unwrap())?;
    fs::write(config_file, &api_key)?;
    Ok(api_key)
}

fn prompt_for_api_key(api_name: &str) -> Result<String> {
    print!("Enter your {} API key: ", api_name);
    io::stdout().flush()?;
    let mut api_key = String::new();
    io::stdin().read_line(&mut api_key)?;
    Ok(api_key.trim().to_string())
}

fn prompt_for_user_input() -> Result<String> {
    print!("Enter your prompt: ");
    io::stdout().flush()?;
    let mut prompt = String::new();
    io::stdin().read_line(&mut prompt)?;
    Ok(prompt.trim().to_string())
}

async fn send_request_to_openrouter(
    api_key: &str, 
    context: &str, 
    model: &str, 
    file_path: &str,
    verbose: bool
) -> Result<Option<String>> {
    let client = Client::new();
    let url = "https://openrouter.ai/api/v1/chat/completions";

    let language = get_file_language(file_path);
    let user_message = format!("The following code is in {}. {}", language, context);

    let request_body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": "You are an assistant helping a developer construct code. Follow instructions carefully and only output the code. Output only the changes, not the entire code."},
            {"role": "user", "content": "add a var sydney to this code | var yemen = 'Middle Eastern country'; var australia = 'Down Under'; function getPopulation(country) { if (country === yemen) { return 30000000; } else if (country === australia) { return 25000000; } else { return 'Unknown'; } }"},
            {"role": "assistant", "content": "```javascript\nvar yemen = 'Middle Eastern country';\nvar australia = 'Down Under';\nvar sydney = 'Largest city in Australia';\n\nfunction getPopulation(country) {\n    if (country === yemen) {\n        return 30000000;\n    } else if (country === australia) {\n        return 25000000;\n    } else if (country === sydney) {\n        return 5000000;\n    } else {\n        return 'Unknown';\n    }\n}```"},
            {"role": "user", "content": "Add a function to calculate factorial in Python | def square(n): return n * n"},
            {"role": "assistant", "content": "```python\ndef square(n): return n * n\ndef factorial(n):\n    if n == 0 or n == 1:\n        return 1\n    else:\n        return n * factorial(n - 1)```"},
            {"role": "user", "content": "Fix the syntax error in this Rust code | fn main() { println(\"Hello, world!\"); }"},
            {"role": "assistant", "content": "```rust\nfn main() {\n    println!(\"Hello, world!\");\n}```"},
            {"role": "user", "content": "Add error handling to this JavaScript function | function divide(a, b) { return a / b; }"},
            {"role": "assistant", "content": "```javascript\nfunction divide(a, b) {\n    if (b === 0) {\n        throw new Error(\"Division by zero\");\n    }\n    return a / b;\n}```"},
            {"role": "user", "content": user_message}
        ],
        "max_tokens": 2048,
        "temperature": 0.7,
        "top_p": 0.9,
    });

    if verbose {
        println!("Sending request to OpenRouter with model: {}", model);
    }
    let spinner = display_waiting_message("Sending request...");

    let response = client.post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    spinner.finish_and_clear();
    if verbose {
        println!("Response status: {}", response.status());
    }

    if response.status().is_success() {
        let spinner = display_waiting_message("Processing response...");
        let body = response.text().await?;
        if verbose {
            println!("Response body: {}", body);
        }
        
        if body.is_empty() {
            spinner.finish_and_clear();
            println!("Received empty response from OpenRouter API");
            return Ok(None);
        }
        
        let json_response: serde_json::Value = serde_json::from_str(&body)?;
        spinner.finish_and_clear();
        
        let content = json_response["choices"][0]["message"]["content"].as_str();
        if content.is_none() {
            println!("Could not extract content from response");
        }
        
        Ok(content.map(String::from))
    } else {
        let error_text = response.text().await?;
        eprintln!("API error response: {}", error_text);
        println!("Error response: {}", error_text);
        Ok(None)
    }
}

fn display_waiting_message(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("▰▱")
            .template("{spinner:.blue} {msg}")
            .unwrap()
    );

    spinner.set_message(message.blue().to_string());
    spinner.enable_steady_tick(Duration::from_millis(100));

    spinner
}

fn smart_merge(original: &str, new: &str) -> (String, Vec<Change>) {
    let original_lines: Vec<&str> = original.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    // If the number of lines is significantly different, treat it as a full file replacement
    if (new_lines.len() as f32 / original_lines.len() as f32).abs() > 0.5 {
        return full_file_diff(&original_lines, &new_lines);
    }

    let mut updated_lines = original_lines.clone();
    let mut changes = Vec::new();

    // Process matching line ranges
    for (i, (old_line, new_line)) in original_lines.iter().zip(new_lines.iter()).enumerate() {
        if old_line != new_line {
            changes.push(Change {
                change_type: ChangeType::Modify,
                line_number: i + 1,
                content: new_line.to_string(),
            });
            updated_lines[i] = new_line;
        }
    }

    // Process added lines
    for (i, new_line) in new_lines.iter().enumerate().skip(original_lines.len()) {
        changes.push(Change {
            change_type: ChangeType::Insert,
            line_number: i + 1,
            content: new_line.to_string(),
        });
        updated_lines.push(new_line);
    }

    // Process removed lines
    for i in new_lines.len()..original_lines.len() {
        changes.push(Change {
            change_type: ChangeType::Delete,
            line_number: i + 1,
            content: original_lines[i].to_string(),
        });
    }

    (updated_lines.join("\n"), changes)
}

fn full_file_diff(original_lines: &[&str], new_lines: &[&str]) -> (String, Vec<Change>) {
    let mut changes = Vec::new();

    // Compare each line and record the differences
    for (i, line) in new_lines.iter().enumerate() {
        if i < original_lines.len() {
            if line != &original_lines[i] {
                changes.push(Change {
                    change_type: ChangeType::Modify,
                    line_number: i + 1,
                    content: line.to_string(),
                });
            }
        } else {
            changes.push(Change {
                change_type: ChangeType::Insert,
                line_number: i + 1,
                content: line.to_string(),
            });
        }
    }

    // Mark lines that exist in original but not in new as deleted
    for i in new_lines.len()..original_lines.len() {
        changes.push(Change {
            change_type: ChangeType::Delete,
            line_number: i + 1,
            content: original_lines[i].to_string(),
        });
    }

    (new_lines.join("\n"), changes)
}

fn show_diff_and_prompt_for_changes(original: &str, new: &str, file_path: &str, user_prompt: &str) -> std::io::Result<()> {
    let extracted_code = extract_code_from_response(new);
    let (updated_content, changes) = smart_merge(original, &extracted_code);
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = TuiApp::new(original, &extracted_code, changes, user_prompt, file_path);

    // Run the TUI
    let result = run_tui(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Handle the result
    match result {
        Ok(true) => {
            // User accepted changes
            std::fs::write(file_path, updated_content)?;
            println!("Changes applied successfully.");
        }
        Ok(false) => {
            println!("Changes discarded.");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    Ok(())
}

fn run_tui<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut TuiApp,
) -> std::io::Result<bool> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(false),
                KeyCode::Char('y') => return Ok(true),
                KeyCode::Char('n') => return Ok(false),
                KeyCode::Up => app.scroll_up(),
                KeyCode::Down => app.scroll_down(),
                KeyCode::Tab => app.toggle_side(),
                _ => {}
            }
        }
    }
}

fn ui<B: ratatui::backend::Backend>(f: &mut ratatui::Frame<B>, app: &TuiApp) {
    // Create a layout with main area and instruction area
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // For prompt display
            Constraint::Min(1),     // For code panels
            Constraint::Length(3),  // For instructions
        ].as_ref())
        .margin(1)
        .split(f.size());

    // Display prompt at the top
    let prompt_block = Block::default()
        .title("User Prompt")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    
    let prompt_text = Paragraph::new(app.user_prompt)
        .block(prompt_block)
        .wrap(Wrap { trim: true });
    
    f.render_widget(prompt_text, main_chunks[0]);

    // Create a layout with two equal columns for the code panels
    let code_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(main_chunks[1]);

    // Render the original code panel
    render_original_panel(f, app, code_chunks[0]);

    // Render the modified code panel
    render_modified_panel(f, app, code_chunks[1]);

    // Render instructions at the bottom
    let instructions = Paragraph::new("Press: [↑/↓] Scroll | [Tab] Switch panels | [y] Apply changes | [n/q] Discard changes")
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));
    
    f.render_widget(instructions, main_chunks[2]);
}

fn render_original_panel<B: ratatui::backend::Backend>(
    f: &mut ratatui::Frame<B>,
    app: &TuiApp,
    area: Rect,
) {
    // Split the area into header and body
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(area);

    // Create header with prompt
    let header = Paragraph::new(format!("Original Code - {}", app.file_path))
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // Create styled text for original code
    let mut text = Text::default();
    for (i, line) in app.original_code.iter().enumerate() {
        // Check if this line is affected by a change
        let style = if app.changes.iter().any(|change| change.line_number == i + 1) {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        };
        
        text.lines.push(Line::from(Span::styled(format!("{:4} {}", i + 1, line), style)));
    }

    // Create the code paragraph with scrolling
    let code_paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .scroll((app.scroll_position, 0))
        .style(
            if app.selected_side == Side::Left {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else {
                Style::default()
            }
        );
    
    f.render_widget(code_paragraph, chunks[1]);
}

fn render_modified_panel<B: ratatui::backend::Backend>(
    f: &mut ratatui::Frame<B>,
    app: &TuiApp,
    area: Rect,
) {
    // Split the area into header and body
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(area);

    // Create header with file name
    let header = Paragraph::new("Modified Code")
        .style(Style::default().fg(Color::Blue))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // Create styled text for modified code with highlighted changes
    let mut text = Text::default();
    for (i, line) in app.modified_code.iter().enumerate() {
        // Determine the style based on change type
        let style = app.changes.iter()
            .find(|change| change.line_number == i + 1)
            .map(|change| match change.change_type {
                ChangeType::Insert => Style::default().fg(Color::Green),
                ChangeType::Delete => Style::default().fg(Color::Red),
                ChangeType::Modify => Style::default().fg(Color::Yellow),
            })
            .unwrap_or_else(|| Style::default());
        
        text.lines.push(Line::from(Span::styled(format!("{:4} {}", i + 1, line), style)));
    }

    // Create the code paragraph with scrolling
    let code_paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .scroll((app.scroll_position, 0))
        .style(
            if app.selected_side == Side::Right {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else {
                Style::default()
            }
        );
    
    f.render_widget(code_paragraph, chunks[1]);
}

fn extract_code_from_response(response: &str) -> String {
    // Extract code between the first pair of triple backticks
    response.lines()
        .skip_while(|line| !line.starts_with("```"))
        .skip(1)  // Skip the line with the opening backticks
        .take_while(|line| !line.starts_with("```"))
        .collect::<Vec<&str>>()
        .join("\n")
}

fn get_file_language(file_path: &str) -> &'static str {
    let extension = Path::new(file_path)
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