use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::path::Path;
use std::time::SystemTime;

// Default models for OpenRouter
pub const DEFAULT_CLAUDE: &str = "anthropic/claude-3.7-sonnet:beta";
pub const DEFAULT_GEMINI: &str = "google/gemini-2.0-flash-001";
pub const AST_GENERATION_MODEL: &str = "google/gemini-2.0-flash-001"; // Use Gemini for AST generation

/// Generate an AST for the provided code
pub async fn generate_ast(api_key: &str, code: &str, language: &str) -> Result<String> {
    let client = Client::new();
    let url = "https://openrouter.ai/api/v1/chat/completions";

    let request_body = json!({
        "model": AST_GENERATION_MODEL,  // Always use Gemini for AST generation
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

/// Send a code modification request to OpenRouter
pub async fn send_code_modification_request(
    api_key: &str,
    prompt: &str,
    code: &str,
    ast: Option<&str>,
    model: &str,
    language: &str,
) -> Result<String> {
    let client = Client::new();
    let url = "https://openrouter.ai/api/v1/chat/completions";

    // Create context with or without AST
    let context = match ast {
        Some(ast_content) => {
            format!(
                "{}\n\nCode:\n{}\n\nAbstract Syntax Tree:\n{}", 
                prompt, code, ast_content
            )
        }
        None => format!("{}\n\n{}", prompt, code),
    };

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

    let response = client.post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    if response.status().is_success() {
        let body = response.text().await?;
        
        if body.is_empty() {
            return Err(anyhow::anyhow!("Received empty response from OpenRouter API"));
        }
        
        let json_response: serde_json::Value = serde_json::from_str(&body)?;
        
        let content = json_response["choices"][0]["message"]["content"]
            .as_str()
            .context("Could not extract content from response")?;
        
        Ok(content.to_string())
    } else {
        let error_text = response.text().await?;
        Err(anyhow::anyhow!("API error response: {}", error_text))
    }
}

/// Extract code from a response that might contain markdown backticks
pub fn extract_code_from_response(response: &str) -> String {
    // Extract code between the first pair of triple backticks
    response.lines()
        .skip_while(|line| !line.starts_with("```"))
        .skip(1)  // Skip the line with the opening backticks
        .take_while(|line| !line.starts_with("```"))
        .collect::<Vec<&str>>()
        .join("\n")
}

/// Get the programming language from a file extension
pub fn get_file_language(file_path: &Path) -> &'static str {
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

/// Validate API key with OpenRouter
pub async fn validate_api_key(api_key: &str) -> Result<bool> {
    let client = Client::new();
    let url = "https://openrouter.ai/api/v1/models";

    let response = client.get(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let is_valid = response.status().is_success();
    Ok(is_valid)
}

/// Save API key to configuration
pub fn save_api_key(api_key: &str) -> Result<()> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?;
    let config_file = config_dir.join("code_ai_openrouter_api_key.txt");
    
    std::fs::create_dir_all(&config_dir)?;
    std::fs::write(config_file, api_key)?;
    
    Ok(())
}

/// Save preferred model to configuration
pub fn save_preferred_model(model: &str) -> Result<()> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?;
    let config_file = config_dir.join("code_ai_preferred_model.txt");
    
    std::fs::create_dir_all(&config_dir)?;
    std::fs::write(config_file, model)?;
    
    Ok(())
}

/// Send a code modification request with additional context from multiple files
pub async fn send_code_modification_request_with_context(
    api_key: &str,
    prompt: &str,
    primary_code: &str,
    additional_context: &str,
    ast: Option<&str>,
    model: &str,
    language: &str,
) -> Result<String> {
    let client = Client::new();
    let url = "https://openrouter.ai/api/v1/chat/completions";

    // Create context with or without AST
    let context = match ast {
        Some(ast_content) => {
            format!(
                "{}\n\nPrimary Code:\n{}\n\nAdditional Context:\n{}\n\nAbstract Syntax Tree:\n{}", 
                prompt, primary_code, additional_context, ast_content
            )
        }
        None => format!(
            "{}\n\nPrimary Code:\n{}\n\nAdditional Context:\n{}", 
            prompt, primary_code, additional_context
        ),
    };

    let user_message = format!("The following primary code is in {}. I'm also providing additional context from other files. Please modify only the primary code based on my request. {}", language, context);

    let request_body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": "You are an assistant helping a developer construct code. Follow instructions carefully and only output the code for the primary file that needs to be modified. Output only the changes, not the entire code."},
            {"role": "user", "content": "I have multiple files. The primary file is a JavaScript router: | const express = require('express'); const router = express.Router(); router.get('/', (req, res) => { res.send('Hello World!'); }); module.exports = router; | And here's my user model: | const mongoose = require('mongoose'); const userSchema = new mongoose.Schema({ name: String, email: String }); module.exports = mongoose.model('User', userSchema); | Add a route to get all users in the router file."},
            {"role": "assistant", "content": "```javascript\nconst express = require('express');\nconst router = express.Router();\nconst User = require('./models/user');\n\nrouter.get('/', (req, res) => {\n  res.send('Hello World!');\n});\n\nrouter.get('/users', async (req, res) => {\n  try {\n    const users = await User.find();\n    res.json(users);\n  } catch (err) {\n    res.status(500).json({ message: err.message });\n  }\n});\n\nmodule.exports = router;```"},
            {"role": "user", "content": user_message}
        ],
        "max_tokens": 2048,
        "temperature": 0.7,
        "top_p": 0.9,
    });

    let response = client.post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    if response.status().is_success() {
        let body = response.text().await?;
        
        if body.is_empty() {
            return Err(anyhow::anyhow!("Received empty response from OpenRouter API"));
        }
        
        let json_response: serde_json::Value = serde_json::from_str(&body)?;
        
        let content = json_response["choices"][0]["message"]["content"]
            .as_str()
            .context("Could not extract content from response")?;
        
        Ok(content.to_string())
    } else {
        let error_text = response.text().await?;
        Err(anyhow::anyhow!("API error response: {}", error_text))
    }
}

/// Fetch credits information from OpenRouter
pub async fn fetch_openrouter_credits(api_key: &str) -> Result<crate::app::CreditsInfo> {
    let client = Client::new();
    let url = "https://openrouter.ai/api/v1/credits";

    let response = client.get(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if response.status().is_success() {
        let body = response.text().await?;
        
        if body.is_empty() {
            return Err(anyhow::anyhow!("Received empty response from OpenRouter API"));
        }
        
        let json_response: serde_json::Value = serde_json::from_str(&body)?;
        
        // Extract credits information
        let total_credits = json_response["data"]["total_credits"]
            .as_f64()
            .context("Could not extract total_credits from response")?;
            
        let total_usage = json_response["data"]["usage"]
            .as_f64()
            .context("Could not extract usage from response")?;
        
        Ok(crate::app::CreditsInfo {
            total_credits,
            total_usage,
            last_updated: SystemTime::now(),
        })
    } else {
        let error_text = response.text().await?;
        Err(anyhow::anyhow!("API error response: {}", error_text))
    }
} 