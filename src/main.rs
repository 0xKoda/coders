use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use reqwest::Client;
use serde_json::json;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use indicatif::{ProgressBar, ProgressStyle};
use colored::*;
use std::time::Duration;
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, ValueEnum)]
enum OpenRouterModel {
    #[value(name = "nousresearch/hermes-3-llama-3.1-405b")]
    NousHermes3Llama31405B,
    #[value(name = "nousresearch/hermes-3-llama-3.1-405b:extended")]
    NousHermes3Llama31405BExtended,
    #[value(name = "meta-llama/llama-3.1-8b-instruct:free")]
    MetaLlama318BInstructFree,
}

#[derive(Clone, ValueEnum)]
enum HyperbolicModel {
    #[value(name = "nous-hermes-3-llama-3-1-70b")]
    NousHermes3Llama3170B,
    #[value(name = "meta-llama-3-1-70b-instruct")]
    MetaLlama3170BInstruct,
    #[value(name = "meta-llama-3-1-8b-instruct")]
    MetaLlama318BInstruct,
    #[value(name = "meta-llama-3-1-405b-instruct")]
    MetaLlama31405BInstruct,
    #[value(name = "meta-llama-3-1-405b")]
    MetaLlama31405B,
}

impl OpenRouterModel {
    fn as_str(&self) -> &'static str {
        match self {
            OpenRouterModel::NousHermes3Llama31405B => "nousresearch/hermes-3-llama-3.1-405b",
            OpenRouterModel::NousHermes3Llama31405BExtended => "nousresearch/hermes-3-llama-3.1-405b:extended",
            OpenRouterModel::MetaLlama318BInstructFree => "meta-llama/llama-3.1-8b-instruct:free",
        }
    }

    fn all() -> Vec<OpenRouterModel> {
        vec![
            OpenRouterModel::NousHermes3Llama31405B,
            OpenRouterModel::NousHermes3Llama31405BExtended,
            OpenRouterModel::MetaLlama318BInstructFree,
        ]
    }
}

impl HyperbolicModel {
    fn as_str(&self) -> &'static str {
        match self {
            HyperbolicModel::NousHermes3Llama3170B => "NousResearch/Hermes-3-Llama-3.1-70B",
            HyperbolicModel::MetaLlama3170BInstruct => "meta-llama/Meta-Llama-3.1-70B-Instruct",
            HyperbolicModel::MetaLlama318BInstruct => "meta-llama/Meta-Llama-3.1-8B-Instruct",
            HyperbolicModel::MetaLlama31405BInstruct => "meta-llama/Meta-Llama-3.1-405B-Instruct",
            HyperbolicModel::MetaLlama31405B => "meta-llama/Meta-Llama-3.1-405B",
        }
    }

    fn all() -> Vec<HyperbolicModel> {
        vec![
            HyperbolicModel::NousHermes3Llama3170B,
            HyperbolicModel::MetaLlama3170BInstruct,
            HyperbolicModel::MetaLlama318BInstruct,
            HyperbolicModel::MetaLlama31405BInstruct,
            HyperbolicModel::MetaLlama31405B,
        ]
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    file: String,
    #[arg(short, long)]
    model: bool,
    #[arg(short, long)]
    openrouter: bool,
}

fn select_model(is_openrouter: bool) -> Result<String> {
    println!("Select a model:");
    if is_openrouter {
        for (i, model) in OpenRouterModel::all().iter().enumerate() {
            println!("{}. {}", i + 1, model.as_str());
        }
    } else {
        for (i, model) in HyperbolicModel::all().iter().enumerate() {
            println!("{}. {}", i + 1, model.as_str());
        }
    }

    loop {
        print!("Enter the number of your choice: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if let Ok(choice) = input.trim().parse::<usize>() {
            if is_openrouter {
                if choice > 0 && choice <= OpenRouterModel::all().len() {
                    return Ok(OpenRouterModel::all()[choice - 1].as_str().to_string());
                }
            } else {
                if choice > 0 && choice <= HyperbolicModel::all().len() {
                    return Ok(HyperbolicModel::all()[choice - 1].as_str().to_string());
                }
            }
        }

        println!("Invalid choice. Please try again.");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let api_key = if cli.openrouter {
        get_or_prompt_for_api_key("OpenRouter").await?
    } else {
        get_or_prompt_for_api_key("Hyperbolic").await?
    };

    let file_content = fs::read_to_string(&cli.file)
        .with_context(|| format!("Failed to read file: {}", cli.file))?;

    let prompt = prompt_for_user_input()?;
    let context = format!("{}\n\n{}", prompt, file_content);

    let model = if cli.model {
        select_model(cli.openrouter)?
    } else if cli.openrouter {
        OpenRouterModel::NousHermes3Llama31405B.as_str().to_string()
    } else {
        HyperbolicModel::MetaLlama31405BInstruct.as_str().to_string()
    };

    let response = if cli.openrouter {
        send_request_to_openrouter(&api_key, &context, &model, &cli.file).await?
    } else {
        send_request_to_hyperbolic(&api_key, &context, &model, &cli.file).await?
    };

    match response {
        Some(content) => {
            println!("API Response:\n{}", content);
            show_diff_and_prompt_for_changes(&file_content, &content, &cli.file)?;
        }
        None => {
            println!("No response received from the API.");
        }
    }

    Ok(())
}

async fn get_or_prompt_for_api_key(api_name: &str) -> Result<String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?;
    let config_file = config_dir.join(format!("{}_api_key.txt", api_name.to_lowercase()));

    println!("Checking for {} API key at: {:?}", api_name, config_file);

    let api_key = if config_file.exists() {
        println!("Found existing {} API key file", api_name);
        let api_key = fs::read_to_string(&config_file)?;
        if api_key.trim().is_empty() {
            println!("Existing {} API key file is empty", api_name);
            prompt_and_save_api_key(api_name, &config_file)?
        } else {
            api_key.trim().to_string()
        }
    } else {
        println!("No existing {} API key file found", api_name);
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
        "Hyperbolic" => "https://api.hyperbolic.xyz/v1/models",
        "OpenRouter" => "https://openrouter.ai/api/v1/models",
        _ => return Err(anyhow::anyhow!("Unknown API provider")),
    };

    let response = client.get(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    Ok(response.status().is_success())
}

fn prompt_and_save_api_key(api_name: &str, config_file: &PathBuf) -> Result<String> {
    let api_key = prompt_for_api_key(api_name)?;
    fs::create_dir_all(config_file.parent().unwrap())?;
    fs::write(config_file, &api_key)?;
    println!("{} API key saved successfully", api_name);
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

async fn send_request_to_hyperbolic(api_key: &str, context: &str, model: &str, file_path: &str) -> Result<Option<String>> {
    let client = Client::new();
    let url = if model == "meta-llama/Meta-Llama-3.1-405B" {
        "https://api.hyperbolic.xyz/v1/completions"
    } else {
        "https://api.hyperbolic.xyz/v1/chat/completions"
    };

    let language = get_file_language(file_path);
    let user_message = format!("The following code is in {}. {}", language, context);

    let request_body = if model == "meta-llama/Meta-Llama-3.1-405B" {
        json!({
            "model": model,
            "prompt": user_message,
            "max_tokens": 512,
            "temperature": 0.7,
            "top_p": 0.9,
            "stream": false
        })
    } else {
        json!({
            "model": model,
            "messages": [
                {"role": "system", "content": "You are an assistant helping a developer construct code. Follow instructions carefully and only output the code. Output only the changes, not the entire code"},
                {"role": "user", "content": "add a var sydney to this code | var yemen = yemen "},
                {"role": "assistant", "content": "```javascript\nvar yemen = yemen;\nvar sydney = sydney;```"},
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
            "stream": false
        })
    };

    println!("Sending request to Hyperbolic API: {}", url);
    println!("Request body: {}", serde_json::to_string_pretty(&request_body)?);

    let spinner = display_waiting_message();

    let response = client.post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    spinner.finish_and_clear();

    println!("Response status: {}", response.status());

    let body = response.text().await?;
    println!("Response body: {}", body);

    if body.is_empty() {
        println!("Received empty response from Hyperbolic API");
        return Ok(None);
    }

    let json_response: serde_json::Value = serde_json::from_str(&body)?;

    Ok(json_response["choices"][0]["text"].as_str().map(String::from))
}

async fn send_request_to_openrouter(api_key: &str, context: &str, model: &str, file_path: &str) -> Result<Option<String>> {
    let client = Client::new();
    let url = "https://openrouter.ai/api/v1/chat/completions";

    let language = get_file_language(file_path);
    let user_message = format!("The following code is in {}. {}", language, context);

    let request_body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": "You are an assistant helping a developer construct code. As you are a machine, you can only reply with code. Follow instructions carefully and only output the code. Output only the changes, not the entire code"},
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

    println!("Sending request to OpenRouter API: {}", url);
    println!("Request body: {}", serde_json::to_string_pretty(&request_body)?);

    let spinner = display_waiting_message();

    let response = client.post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    spinner.finish_and_clear();

    println!("Response status: {}", response.status());

    let body = response.text().await?;
    println!("Response body: {}", body);

    if body.is_empty() {
        println!("Received empty response from OpenRouter API");
        return Ok(None);
    }

    let json_response: serde_json::Value = serde_json::from_str(&body)?;

    Ok(json_response["choices"][0]["message"]["content"].as_str().map(String::from))
}

fn display_waiting_message() -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("▰▱")
            .template("{spinner:.blue} {msg}")
            .unwrap()
    );

    spinner.set_message("Awaiting response...".blue().to_string());
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

    // Handle added lines
    for (i, new_line) in new_lines.iter().enumerate().skip(original_lines.len()) {
        changes.push(Change {
            change_type: ChangeType::Insert,
            line_number: i + 1,
            content: new_line.to_string(),
        });
        updated_lines.push(new_line);
    }

    // Handle deleted lines
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
    println!("\nProposed changes:");
    println!("------------------");

    let extracted_code = extract_code_from_response(new);
    let (updated_content, changes) = smart_merge(original, &extracted_code);

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
    response.lines()
        .skip_while(|line| !line.starts_with("```"))
        .skip(1)
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