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
    #[arg(short, long)]
    openrouter: bool,
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
    #[value(name = "nousresearch-hermes-3-llama-3-1-405b")]
    NousHermes3Llama31405B,
}

impl Model {
    fn as_str(&self) -> &'static str {
        match self {
            Model::NousHermes3Llama3170B => "NousResearch/Hermes-3-Llama-3.1-70B",
            Model::MetaLlama3170BInstruct => "meta-llama/Meta-Llama-3.1-70B-Instruct",
            Model::MetaLlama318BInstruct => "meta-llama/Meta-Llama-3.1-8B-Instruct",
            Model::MetaLlama370BInstruct => "meta-llama/Meta-Llama-3-70B-Instruct",
            Model::MetaLlama31405BInstruct => "meta-llama/Meta-Llama-3.1-405B-Instruct",
            Model::NousHermes3Llama31405B => "nousresearch/hermes-3-llama-3.1-405b",
        }
    }

    fn all() -> Vec<Model> {
        vec![
            Model::NousHermes3Llama3170B,
            Model::MetaLlama3170BInstruct,
            Model::MetaLlama318BInstruct,
            Model::MetaLlama370BInstruct,
            Model::MetaLlama31405BInstruct,
            Model::NousHermes3Llama31405B,
        ]
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let hyperbolic_api_key = get_or_prompt_for_api_key("Hyperbolic")?;
    let openrouter_api_key = get_or_prompt_for_api_key("OpenRouter")?;

    let file_content = fs::read_to_string(&cli.file)
        .with_context(|| format!("Failed to read file: {}", cli.file))?;

    let prompt = prompt_for_user_input()?;
    let context = format!("{}\n\n{}", prompt, file_content);

    let model = if cli.model {
        select_model()?
    } else if cli.openrouter {
        Model::NousHermes3Llama31405B
    } else {
        Model::MetaLlama31405BInstruct
    };

    let response = if cli.openrouter {
        send_request_to_openrouter(&openrouter_api_key, &context, model).await?
    } else {
        send_request_to_hyperbolic(&hyperbolic_api_key, &context, model).await?
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

fn get_or_prompt_for_api_key(api_name: &str) -> Result<String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?;
    let config_file = config_dir.join(format!("{}_api_key.txt", api_name.to_lowercase()));

    if config_file.exists() {
        return Ok(fs::read_to_string(&config_file)?);
    }

    let api_key = prompt_for_api_key(api_name)?;
    fs::create_dir_all(config_file.parent().unwrap())?;
    fs::write(&config_file, &api_key)?;
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

async fn send_request_to_hyperbolic(api_key: &str, context: &str, model: Model) -> Result<Option<String>> {
    let client = Client::new();
    let url = "https://api.hyperbolic.xyz/v1/chat/completions";

    let request_body = json!({
        "model": model.as_str(),
        "messages": [
            {"role": "system", "content": "You are an assistant helping a developer construct code, follow instructions carefully and only output the code, if you must output words (you must not) do so inside //"},
            {
                "role": "user",
                "content": context
            }
        ],
        "max_tokens": 2048,
        "temperature": 0.7,
        "top_p": 0.9,
        "stream": false
    });

    println!("Sending request to Hyperbolic API: {}", url);
    println!("Request body: {}", serde_json::to_string_pretty(&request_body)?);

    let response = client.post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    println!("Response status: {}", response.status());

    let body = response.text().await?;
    println!("Response body: {}", body);

    if body.is_empty() {
        println!("Received empty response from Hyperbolic API");
        return Ok(None);
    }

    let json_response: serde_json::Value = serde_json::from_str(&body)?;

    Ok(json_response["choices"][0]["message"]["content"].as_str().map(String::from))
}

async fn send_request_to_openrouter(api_key: &str, context: &str, model: Model) -> Result<Option<String>> {
    let client = Client::new();
    let url = "https://openrouter.ai/api/v1/chat/completions";

    let request_body = json!({
        "model": model.as_str(),
        "messages": [
            {"role": "system", "content": "You are an assistant helping a developer construct code, follow instructions carefully and only output the code"},
            {
                "role": "user",
                "content": context
            }
        ],
        "max_tokens": 2048,
        "temperature": 0.7,
        "top_p": 0.9,
    });

    println!("Sending request to OpenRouter API: {}", url);
    println!("Request body: {}", serde_json::to_string_pretty(&request_body)?);

    let response = client.post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

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