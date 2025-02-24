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

// Default models for OpenRouter
const DEFAULT_CLAUDE: &str = "anthropic/claude-3.7-sonnet:beta";
const DEFAULT_GEMINI: &str = "google/gemini-2.0-flash-001";

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

    // Get user prompt
    let prompt = prompt_for_user_input()?;
    let context = format!("{}\n\n{}", prompt, file_content);

    // Determine which model to use
    let model = determine_model(&cli)?;
    println!("Using model: {}", model);

    // Send request to OpenRouter
    let response = send_request_to_openrouter(&api_key, &context, &model, file_path, cli.verbose).await?;

    match response {
        Some(content) => {
            println!("API Response:\n{}", content);
            show_diff_and_prompt_for_changes(&file_content, &content, file_path)?;
        }
        None => {
            eprintln!("No valid response received from the API.");
            println!("No valid response received from the API.");
        }
    }

    Ok(())
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

fn show_diff_and_prompt_for_changes(original: &str, new: &str, file_path: &str) -> std::io::Result<()> {
    let extracted_code = extract_code_from_response(new);
    let (updated_content, changes) = smart_merge(original, &extracted_code);

    println!("\nProposed changes:");
    println!("------------------");

    // Display changes with color coding
    for change in &changes {
        match change.change_type {
            ChangeType::Insert => println!("\x1b[32m+ {}:{}\x1b[0m", change.line_number, change.content),
            ChangeType::Delete => println!("\x1b[31m- {}:{}\x1b[0m", change.line_number, change.content),
            ChangeType::Modify => println!("\x1b[33m~ {}:{}\x1b[0m", change.line_number, change.content),
        }
    }

    println!("\nDo you want to apply these changes? (y/n)");
    std::io::stdout().flush()?;

    let mut response = String::new();
    std::io::stdin().read_line(&mut response)?;

    if response.trim().to_lowercase() == "y" {
        std::fs::write(file_path, updated_content)?;
        println!("Changes applied successfully.");
    } else {
        println!("Changes discarded.");
    }

    Ok(())
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