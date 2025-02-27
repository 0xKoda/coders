use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::path::{Path, PathBuf};
use log::{debug, info, error};
use serde::Deserialize;

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
    info!("Sending code modification request to model: {}", model);
    debug!("Language: {}, Code length: {} bytes", language, code.len());
    debug!("Prompt: {}", prompt);
    
    // Construct a prompt message that includes instructions for modifying code
    let system_message = format!(
        "You are a helpful AI coding assistant. You will be given a user request and their code in {}. 
        Your task is to modify the code according to the user's request.
        Respond with both an explanation of the changes and the modified code.
        
        Guidelines:
        1. Always respond with the full, modified version of the original code
        2. Wrap the code in triple backticks (```) with the language name
        3. Explain what changes you made and why
        4. If the user wants a completely new implementation, provide a full solution
        5. Use best practices and write efficient, clean code",
        language
    );
    
    // Create full user message with prompt and code
    let user_message = format!(
        "Here is my code in {}:\n\n```\n{}\n```\n\nRequest: {}\n\nPlease provide a full, modified version of the code that addresses my request, along with an explanation of your changes.",
        language, code, prompt
    );
    
    // Add AST information if available
    let user_message = if let Some(ast_data) = ast {
        format!("{}\n\nHere is the AST for the code:\n```\n{}\n```", user_message, ast_data)
    } else {
        user_message
    };
    
    // Create request payload
    let payload = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": system_message
            },
            {
                "role": "user", 
                "content": user_message
            }
        ],
        "temperature": get_model_temperature(model),
        "max_tokens": 8000
    });
    
    debug!("Sending request to OpenRouter with payload length: {} bytes", 
           serde_json::to_string(&payload)?.len());
    
    // Set up headers
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION, 
        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key))?
    );
    headers.insert(reqwest::header::CONTENT_TYPE, reqwest::header::HeaderValue::from_static("application/json"));
    headers.insert("HTTP-Referer", reqwest::header::HeaderValue::from_static("https://github.com/yourname/code-ai"));
    headers.insert("X-Title", reqwest::header::HeaderValue::from_static("Code-AI Assistant"));
    
    // Send request
    let client = reqwest::Client::new();
    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .headers(headers)
        .json(&payload)
        .send()
        .await?;
    
    // Check response status
    if !response.status().is_success() {
        let error_text = response.text().await?;
        error!("OpenRouter API error: {}", error_text);
        return Err(anyhow::anyhow!("API error: {}", error_text));
    }
    
    // Parse response
    let response_text = response.text().await?;
    debug!("Received response from OpenRouter: {} bytes", response_text.len());
    
    // Create a sanitized version for logging (truncated to avoid giant log files)
    let max_log_length = 1000;
    let truncated_response = if response_text.len() > max_log_length {
        format!("{}... [truncated, total length: {}]", 
                &response_text[0..max_log_length], 
                response_text.len())
    } else {
        response_text.clone()
    };
    
    debug!("Response content: {}", truncated_response);
    
    #[derive(Deserialize)]
    struct OpenRouterResponse {
        choices: Vec<OpenRouterChoice>,
    }
    
    #[derive(Deserialize)]
    struct OpenRouterChoice {
        message: OpenRouterMessage,
    }
    
    #[derive(Deserialize)]
    struct OpenRouterMessage {
        content: String,
    }
    
    let response_data: OpenRouterResponse = match serde_json::from_str(&response_text) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to parse OpenRouter response: {}", e);
            return Err(anyhow::anyhow!("Failed to parse API response: {}", e));
        }
    };
    
    // Get the content from the first choice
    if let Some(choice) = response_data.choices.first() {
        info!("Successfully received code modification response");
        Ok(choice.message.content.clone())
    } else {
        error!("No choices in the API response");
        Err(anyhow::anyhow!("No content in the API response"))
    }
}

/// Process a response that may contain multiple file edits
fn process_multi_file_response_internal(response: &str) -> Vec<(String, String)> {
    // Log the response we're trying to process for multiple files
    log::info!("Processing multi-file response: {}", response);
    
    let mut file_edits = Vec::new();
    
    // Check if the response contains multiple file sections
    if response.contains("File:") || response.contains("file:") {
        // Log that we detected a potential multi-file response
        log::info!("Detected potential multi-file response");
        
        // Extract multiple files from the response
        file_edits = extract_multiple_files_from_response(response);
        
        // Log what we found
        log::info!("Extracted {} files from response", file_edits.len());
        for (file, content) in &file_edits {
            log::info!("Found file in response: {} with content length: {}", file, content.len());
        }
    } else {
        log::info!("Response does not appear to contain multiple files");
    }
    
    file_edits
}

/// Calculate diff between original and modified code
fn calculate_diff(original: &str, modified: &str) -> Vec<crate::app::Change> {
    let mut changes = Vec::new();
    
    // Split the original and modified code into lines
    let original_lines: Vec<&str> = original.lines().collect();
    let modified_lines: Vec<&str> = modified.lines().collect();
    
    // Use a simple line-by-line comparison for now
    // This is a basic implementation and could be improved with a proper diff algorithm
    let max_lines = std::cmp::max(original_lines.len(), modified_lines.len());
    
    for i in 0..max_lines {
        let original_line = original_lines.get(i).map(|s| *s).unwrap_or("");
        let modified_line = modified_lines.get(i).map(|s| *s).unwrap_or("");
        
        if i >= original_lines.len() {
            // Line was added
            changes.push(crate::app::Change {
                change_type: crate::app::ChangeType::Insert,
                line_number: i,
                content: modified_line.to_string(),
            });
        } else if i >= modified_lines.len() {
            // Line was deleted
            changes.push(crate::app::Change {
                change_type: crate::app::ChangeType::Delete,
                line_number: i,
                content: original_line.to_string(),
            });
        } else if original_line != modified_line {
            // Line was modified
            changes.push(crate::app::Change {
                change_type: crate::app::ChangeType::Modify,
                line_number: i,
                content: modified_line.to_string(),
            });
        }
    }
    
    changes
}

/// Extract code from the LLM response
pub fn extract_code_from_response(response: &str) -> String {
    // Log the response we're trying to extract code from
    log::info!("Extracting code from response: {}", response);
    
    // Check if the response contains multiple file edits
    if (response.contains("File:") || response.contains("file:")) && response.contains("```") {
        log::info!("Detected multi-file response format, processing accordingly");
        
        // Process the multi-file response and get the files
        let files = process_multi_file_response_internal(response);
        
        // If we found files, return the content of the first one
        if !files.is_empty() {
            log::info!("Returning content of first file: {}", files[0].0);
            return files[0].1.clone();
        }
    }
    
    // Regular single file extraction
    log::info!("Falling back to single file extraction");
    let mut code = String::new();
    let mut in_code_block = false;
    let mut language_line = false;
    
    for line in response.lines() {
        if line.trim().starts_with("```") {
            if in_code_block {
                in_code_block = false;
            } else {
                in_code_block = true;
                language_line = true;
                continue;
            }
        } else if in_code_block {
            if language_line {
                language_line = false;
                // Skip language identifier line
                continue;
            } else {
                code.push_str(line);
                code.push('\n');
            }
        }
    }
    
    log::info!("Extracted code length: {}", code.len());
    code
}

/// Extract explanation text from a response
fn extract_explanation_text(response: &str) -> Option<String> {
    // Log the response we're extracting from
    log::debug!("Extracting explanation from response of length: {}", response.len());
    
    // Split the response into lines
    let lines: Vec<&str> = response.lines().collect();
    
    // Skip initial empty lines
    let mut i = 0;
    while i < lines.len() && lines[i].trim().is_empty() {
        i += 1;
    }
    
    // Skip lines that might be part of markdown formatting at the beginning
    // like "```" if it's the first line
    if i < lines.len() && lines[i].trim() == "```" {
        i += 1;
    }
    
    let mut explanation = Vec::new();
    
    // Collect lines until we hit a code block or file marker
    while i < lines.len() {
        let line = lines[i].trim();
        
        // Break if we hit a code block or file marker
        if line.starts_with("```") || 
           line.starts_with("File:") || 
           line.starts_with("file:") {
            break;
        }
        
        explanation.push(lines[i]);
        i += 1;
    }
    
    // If we collected any explanation lines, join them and return
    if !explanation.is_empty() {
        let result = explanation.join("\n");
        log::info!("Extracted explanation of length: {}", result.len());
        Some(result)
    } else {
        // If we didn't find any explanation at the beginning, look for text between code blocks
        log::info!("No explanation found at beginning, looking between code blocks");
        
        let mut i = 0;
        let mut in_code_block = false;
        let mut between_blocks_text = Vec::new();
        
        while i < lines.len() {
            let line = lines[i].trim();
            
            if line.starts_with("```") {
                in_code_block = !in_code_block;
                
                // If we just ended a code block, start collecting text
                if !in_code_block {
                    let mut j = i + 1;
                    let mut block_text = Vec::new();
                    
                    // Collect lines until the next code block or file marker
                    while j < lines.len() {
                        let next_line = lines[j].trim();
                        if next_line.starts_with("```") || 
                           next_line.starts_with("File:") || 
                           next_line.starts_with("file:") {
                            break;
                        }
                        
                        if !next_line.is_empty() {
                            block_text.push(lines[j]);
                        }
                        j += 1;
                    }
                    
                    // If we found text, add it to our collection
                    if !block_text.is_empty() {
                        between_blocks_text.extend(block_text);
                    }
                }
            }
            
            i += 1;
        }
        
        // If we found text between code blocks, return it
        if !between_blocks_text.is_empty() {
            let result = between_blocks_text.join("\n");
            log::info!("Extracted explanation between code blocks, length: {}", result.len());
            Some(result)
        } else {
            log::info!("No explanation found in response");
            None
        }
    }
}

/// Process a response from the API
pub async fn process_response(
    response: String,
    original_content: &str,
    _file_path: &Path,
) -> Result<crate::app::FileDiff> {
    // Extract code from the response
    let code = extract_code_from_response(&response);
    
    // Extract explanation text
    let explanation_text = extract_explanation_text(&response);
    
    // Calculate the diff between the original and modified code
    let changes = calculate_diff(original_content, &code);
    
    // Create a FileDiff struct
    let diff = crate::app::FileDiff {
        original: original_content.to_string(),
        modified: code,
        changes,
        explanation_text,
    };
    
    Ok(diff)
}

/// Process a multi-file response
pub async fn process_multi_file_response(
    response: String,
    files_content: &[(PathBuf, String)],
) -> Result<Vec<(String, crate::app::FileDiff)>> {
    // Extract files from the response
    let files = extract_multiple_files_from_response(&response);
    
    // Log the extracted files for debugging
    log::info!("Extracted {} files from response", files.len());
    for (file_name, content) in &files {
        log::info!("  - {} (content length: {})", file_name, content.len());
    }
    
    // Log the available files for matching
    log::info!("Available files for matching: {}", files_content.len());
    for (path, _) in files_content {
        log::info!("  - {:?}", path);
    }
    
    // Extract explanation text (common for all files)
    let explanation_text = extract_explanation_text(&response);
    if let Some(ref exp) = explanation_text {
        log::info!("Extracted explanation text (length: {})", exp.len());
    } else {
        log::info!("No explanation text extracted");
    }
    
    let mut result = Vec::new();
    
    // Process each file
    for (file_name, file_content) in files {
        // Find the original content for this file using various matching strategies
        let normalized_name = if file_name.starts_with("./") {
            file_name[2..].to_string()
        } else {
            file_name.clone()
        };
        
        let basename = Path::new(&normalized_name)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| normalized_name.clone());
            
        // Try multiple matching strategies
        let original_content = files_content
            .iter()
            .find(|(path, _)| {
                let path_str = path.to_string_lossy().to_string();
                
                // Exact path match
                if path_str == normalized_name {
                    log::info!("Found exact path match for {}: {:?}", file_name, path);
                    return true;
                }
                
                // Path ends with the filename
                if path_str.ends_with(&normalized_name) || 
                   path_str.ends_with(&format!("/{}", normalized_name)) {
                    log::info!("Found path ending with {} for {}: {:?}", normalized_name, file_name, path);
                    return true;
                }
                
                // Basename match
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy() == basename {
                        log::info!("Found basename match ({}) for {}: {:?}", basename, file_name, path);
                        return true;
                    }
                }
                
                false
            })
            .map(|(_, content)| content.clone())
            .unwrap_or_else(|| {
                // Log that we couldn't find the original content
                log::warn!("Original content not found for file: {}, using empty string", file_name);
                String::new()
            });
        
        log::info!("File: {}, Original content length: {}, Modified content length: {}", 
                  file_name, original_content.len(), file_content.len());
        
        // Calculate the diff
        let changes = calculate_diff(&original_content, &file_content);
        
        // Create a FileDiff struct
        let diff = crate::app::FileDiff {
            original: original_content,
            modified: file_content,
            changes,
            explanation_text: explanation_text.clone(),
        };
        
        result.push((file_name, diff));
    }
    
    Ok(result)
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

/// Send a code modification request with context from multiple files
pub async fn send_code_modification_request_with_context(
    api_key: &str,
    prompt: &str,
    code: &str,
    context: &str,
    ast: Option<&str>,
    model: &str,
    language: &str,
) -> Result<String> {
    // Create a client
    let client = reqwest::Client::new();
    
    // Construct the system prompt
    let system_prompt = format!(
        r#"You are an expert software developer. Your task is to modify the provided code according to the user's request.

If the user's request involves modifying multiple files, you MUST format your response exactly as follows:

1. Start with a brief summary of the changes you're making
2. For each file that needs changes, include:
   - A line that says "File: filename.ext" (use the exact filename)
   - Immediately followed by a code block with triple backticks
   - The complete updated content of the file inside the code block
   - Close the code block with triple backticks

IMPORTANT: Each file section MUST follow this exact pattern:
File: filename.ext
```
[complete file content here]
```

Example multi-file response format:
```
I've made the following changes:
1. Updated the function in main.js
2. Modified the helper function in utils.js

File: main.js
```javascript
// Complete content of main.js with changes
```

File: utils.js
```javascript
// Complete content of utils.js with changes
```
```

If only one file needs changes, you can simply return the modified code in a code block.

The primary file you need to modify is written in {language}. Focus on making the requested changes while maintaining the overall structure and style of the code.

DO NOT include any explanations or comments outside of the code blocks unless absolutely necessary for clarity. The code should be ready to use without any modifications."#
    );
    
    // Log the system prompt
    log::debug!("System prompt: {}", system_prompt);
    
    // Create the user prompt with code and context
    let user_prompt = if !context.is_empty() {
        format!(
            "I need to modify the following code based on this request: {}\n\nPRIMARY FILE TO MODIFY:\n```\n{}\n```\n\nCONTEXT FROM RELATED FILES:{}\n\nPlease provide the complete modified code for any files that need changes.",
            prompt, code, context
        )
    } else {
        format!(
            "I need to modify the following code based on this request: {}\n\n```\n{}\n```\n\nPlease provide the complete modified code.",
            prompt, code
        )
    };
    
    // Add AST if provided
    let user_prompt = if let Some(ast) = ast {
        format!("{}\n\nHere is the AST of the code:\n```\n{}\n```", user_prompt, ast)
    } else {
        user_prompt
    };
    
    // Create the request body
    let request_body = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": system_prompt
            },
            {
                "role": "user",
                "content": user_prompt
            }
        ],
        "temperature": 0.7,
        "max_tokens": 4000
    });
    
    // Log the request for debugging
    log::debug!("Sending request to OpenRouter API: {:?}", request_body);
    
    // Make the API request
    let response = client.post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;
    
    // Check if the request was successful
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("API request failed: {}", error_text));
    }
    
    // Parse the response
    let response_json: serde_json::Value = response.json().await?;
    
    // Extract the response text
    let response_text = response_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to extract response text"))?
        .to_string();
    
    // Log the raw response for debugging
    log::info!("Raw API response: {}", response_text);
    
    Ok(response_text)
}

/// Fetch credits information from OpenRouter
pub async fn fetch_openrouter_credits(api_key: &str) -> Result<crate::app::CreditsInfo> {
    // Create a client with a timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    
    // Make the request to the OpenRouter API
    let response = client.get("https://openrouter.ai/api/v1/credits")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;
    
    // Check if the request was successful
    if !response.status().is_success() {
        let error_text = response.text().await?;
        error!("Failed to fetch credits: {}", error_text);
        return Err(anyhow::anyhow!("Failed to fetch credits: {}", error_text));
    }
    
    // Parse the response JSON
    let response_text = response.text().await?;
    log::debug!("Credits response: {}", response_text);
    
    // Parse the JSON response according to OpenRouter's format
    let response_json: serde_json::Value = serde_json::from_str(&response_text)?;
    
    // Extract the credits information from the response
    // The format is {"data": {"total_credits": 1.1, "total_usage": 1.1}}
    let data = response_json.get("data").ok_or_else(|| anyhow::anyhow!("Missing 'data' field in response"))?;
    
    let total_credits = data.get("total_credits")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'total_credits' field"))?;
    
    let total_usage = data.get("total_usage")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'total_usage' field"))?;
    
    // Create and return the CreditsInfo struct
    Ok(crate::app::CreditsInfo {
        total_credits,
        total_usage,
        last_updated: std::time::SystemTime::now(),
    })
}

/// Extract multiple files from a response
fn extract_multiple_files_from_response(response: &str) -> Vec<(String, String)> {
    let mut file_edits = Vec::new();
    
    // Split the response by lines for processing
    let lines: Vec<&str> = response.lines().collect();
    let mut i = 0;
    
    while i < lines.len() {
        let line = lines[i].trim();
        
        // Log each line for detailed debugging
        log::debug!("Processing line {}: {}", i, line);
        
        // Look for file markers
        if line.contains("File:") || line.contains("file:") {
            // Extract the file name
            let file_name = extract_file_name(line);
            log::info!("Found file marker at line {}: {}", i, file_name);
            
            // Look for the start of a code block
            let mut code_block_start = i + 1;
            while code_block_start < lines.len() && !lines[code_block_start].trim().starts_with("```") {
                code_block_start += 1;
            }
            
            if code_block_start < lines.len() {
                // Found the start of a code block
                let mut code_block_end = code_block_start + 1;
                while code_block_end < lines.len() && !lines[code_block_end].trim().starts_with("```") {
                    code_block_end += 1;
                }
                
                if code_block_end < lines.len() {
                    // Found the end of a code block
                    let mut code_content = String::new();
                    
                    // Skip the language identifier line if present
                    let content_start = if code_block_start + 1 < code_block_end && 
                                         (lines[code_block_start + 1].contains("javascript") || 
                                          lines[code_block_start + 1].contains("python") ||
                                          lines[code_block_start + 1].contains("rust") ||
                                          lines[code_block_start + 1].contains("java") ||
                                          lines[code_block_start + 1].contains("typescript")) {
                        code_block_start + 2
                    } else {
                        code_block_start + 1
                    };
                    
                    // Extract the code content
                    for j in content_start..code_block_end {
                        code_content.push_str(lines[j]);
                        code_content.push('\n');
                    }
                    
                    log::info!("Extracted code for file {} from lines {}-{}, content length: {}", 
                              file_name, content_start, code_block_end, code_content.len());
                    
                    file_edits.push((file_name, code_content));
                    
                    // Move to the end of this code block
                    i = code_block_end;
                }
            }
        }
        
        i += 1;
    }
    
    file_edits
}

/// Helper function to extract file name from a line
fn extract_file_name(line: &str) -> String {
    // Try different formats of file markers
    if let Some(pos) = line.find("File:") {
        let file_part = &line[pos + 5..].trim();
        // Extract until the end of line or until a special character
        if let Some(end) = file_part.find(|c: char| c == '`' || c == ':' || c == '(' || c == ')') {
            file_part[..end].trim().to_string()
        } else {
            file_part.to_string()
        }
    } else if let Some(pos) = line.find("file:") {
        let file_part = &line[pos + 5..].trim();
        if let Some(end) = file_part.find(|c: char| c == '`' || c == ':' || c == '(' || c == ')') {
            file_part[..end].trim().to_string()
        } else {
            file_part.to_string()
        }
    } else if let Some(pos) = line.find("File ") {
        let file_part = &line[pos + 5..].trim();
        if let Some(end) = file_part.find(|c: char| c == '`' || c == ':' || c == '(' || c == ')') {
            file_part[..end].trim().to_string()
        } else {
            file_part.to_string()
        }
    } else if let Some(pos) = line.find("file ") {
        let file_part = &line[pos + 5..].trim();
        if let Some(end) = file_part.find(|c: char| c == '`' || c == ':' || c == '(' || c == ')') {
            file_part[..end].trim().to_string()
        } else {
            file_part.to_string()
        }
    } else {
        // Default case if we can't extract a proper file name
        "unknown_file".to_string()
    }
}

/// Model specific temperature settings
fn get_model_temperature(model: &str) -> f32 {
    match model {
        m if m.contains("gemini") => 0.7, // Gemini models work better with slightly lower temp
        m if m.contains("claude") => 0.7, // Claude models work well with moderate temperature
        _ => 0.7, // Default for other models
    }
} 
