use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use reqwest::Client;
use serde_json::json;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    file: String,
    #[arg(short, long)]
    model: bool,
}

#[derive(Clone, ValueEnum)]
enum Model {
    #[value(name = "nous-hermes-3-llama-3-1-70b")]
    NousHermes3Llama3170B,
    #[value(name = "meta-llama-3-1-70b-instruct")]
    MetaLlama3170BInstruct,
    #[value(name = "meta-llama-3-1-8b-instruct")]
    MetaLlama318BInstruct,
    #[value(name = "meta-llama-3-70b-instruct")]
    MetaLlama370BInstruct,
    #[value(name = "meta-llama-3-1-405b-instruct")]
    MetaLlama31405BInstruct,
}

impl Model {
    fn as_str(&self) -> &'static str {
        match self {
            Model::NousHermes3Llama3170B => "NousResearch/Hermes-3-Llama-3.1-70B",
            Model::MetaLlama3170BInstruct => "meta-llama/Meta-Llama-3.1-70B-Instruct",
            Model::MetaLlama318BInstruct => "meta-llama/Meta-Llama-3.1-8B-Instruct",
            Model::MetaLlama370BInstruct => "meta-llama/Meta-Llama-3-70B-Instruct",
            Model::MetaLlama31405BInstruct => "meta-llama/Meta-Llama-3.1-405B-Instruct",
        }
    }

    fn all() -> Vec<Model> {
        vec![
            Model::NousHermes3Llama3170B,
            Model::MetaLlama3170BInstruct,
            Model::MetaLlama318BInstruct,
            Model::MetaLlama370BInstruct,
            Model::MetaLlama31405BInstruct,
        ]
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let api_key = get_or_prompt_for_api_key()?;
    
    let file_content = fs::read_to_string(&cli.file)
        .with_context(|| format!("Failed to read file: {}", cli.file))?;
    
    let prompt = prompt_for_user_input()?;
    let context = format!("{}\n\n{}", prompt, file_content);
    
    let model = if cli.model {
        select_model()?
    } else {
        Model::MetaLlama31405BInstruct
    };
    
    let response = send_request_to_api(&api_key, &context, model).await?;
    println!("API Response:\n{}", response);
    
    show_diff_and_prompt_for_changes(&file_content, &response, &cli.file)?;
    
    Ok(())
}

fn select_model() -> Result<Model> {
    println!("Select a model:");
    for (i, model) in Model::all().iter().enumerate() {
        println!("{}. {}", i + 1, model.as_str());
    }
    
    loop {
        print!("Enter the number of your choice: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if let Ok(choice) = input.trim().parse::<usize>() {
            if choice > 0 && choice <= Model::all().len() {
                return Ok(Model::all()[choice - 1].clone());
            }
        }
        
        println!("Invalid choice. Please try again.");
    }
}

fn get_or_prompt_for_api_key() -> Result<String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?;
    let config_file = config_dir.join("ai_cli_config.json");

    if config_file.exists() {
        let config_content = fs::read_to_string(&config_file)?;
        let config: serde_json::Value = serde_json::from_str(&config_content)?;
        if let Some(api_key) = config["api_key"].as_str() {
            return Ok(api_key.to_string());
        }
    }

    let api_key = prompt_for_api_key()?;
    save_api_key(&config_file, &api_key)?;
    Ok(api_key)
}

fn prompt_for_api_key() -> Result<String> {
    print!("Enter your API key: ");
    io::stdout().flush()?;
    let mut api_key = String::new();
    io::stdin().read_line(&mut api_key)?;
    Ok(api_key.trim().to_string())
}

fn save_api_key(config_file: &PathBuf, api_key: &str) -> Result<()> {
    let config = json!({
        "api_key": api_key
    });
    let config_content = serde_json::to_string_pretty(&config)?;
    fs::create_dir_all(config_file.parent().unwrap())?;
    fs::write(config_file, config_content)?;
    Ok(())
}

fn prompt_for_user_input() -> Result<String> {
    print!("Enter your prompt: ");
    io::stdout().flush()?;
    let mut prompt = String::new();
    io::stdin().read_line(&mut prompt)?;
    Ok(prompt.trim().to_string())
}

async fn send_request_to_api(api_key: &str, context: &str, model: Model) -> Result<String> {
    let client = Client::new();
    let url = "https://api.hyperbolic.xyz/v1/chat/completions";
    
    let response = client.post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "model": model.as_str(),
            "messages": [ {"role": "system", "content": "You are an assistant helping a developer construct code, follow instructions carefully and only output the code"},
            {
                "role": "user",
                "content": "add a var sydney to this code | var yemen = yemen ",
                "assistant": "var yemen = yemen ;
                var sydney = sydney;"
            }, 
                {
                    "role": "user",
                    "content": context
                }
            ],
            "max_tokens": 2048,
            "temperature": 0.7,
            "top_p": 0.9,
            "stream": false
        }))
        .send()
        .await?;
    
    // Print status code for debug
    // println!("Status: {}", response.status());

    // Get the response body as text
    let body = response.text().await?;
    
    // Print the raw response body for debug
    // println!("Raw response: {}", body);

    // Try to parse as JSON
    let json_response: serde_json::Value = serde_json::from_str(&body)?;
    
    Ok(json_response["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
}

fn show_diff_and_prompt_for_changes(original: &str, new: &str, file_path: &str) -> Result<()> {
    println!("\nProposed changes:");
    println!("------------------");
    
    let original_lines: Vec<&str> = original.lines().collect();
    let new_lines: Vec<&str> = new.lines()
        .filter(|&line| !line.trim_start().starts_with("```"))
        .collect();
    
    let max_lines = original_lines.len().max(new_lines.len());
    
    for i in 0..max_lines {
        let original_line = original_lines.get(i).unwrap_or(&"");
        let new_line = new_lines.get(i).unwrap_or(&"");
        
        if original_line != new_line {
            println!("\x1b[31m- {}\x1b[0m", original_line);
            println!("\x1b[32m+ {}\x1b[0m", new_line);
        } else {
            println!("  {}", original_line);
        }
    }
    
    println!("\nDo you want to apply these changes? (y/n)");
    io::stdout().flush()?;
    
    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    
    if response.trim().to_lowercase() == "y" {
        let updated_content: String = new_lines.join("\n");
        fs::write(file_path, updated_content)?;
        println!("Changes applied successfully.");
    } else {
        println!("Changes discarded.");
    }
    
    Ok(())
}
