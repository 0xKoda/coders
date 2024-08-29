# Coders AI Assistant

Coders is an AI-powered command-line tool that helps you modify and improve your code.
The assistant uses models from hyperbolic.xyz, make sure you have an API key, Hyperbolic supplies free credits for new users.

## Options

- `-f, --file <FILE>`: Specify the file to process (required)
- `-m, --model`: Enable model selection (optional)
- `-h, --help`: Display help information and all available options
- `-V, --version`: Print version information

## Available Models

- NousResearch/Hermes-3-Llama-3.1-70B
- meta-llama/Meta-Llama-3.1-70B-Instruct
- meta-llama/Meta-Llama-3.1-8B-Instruct
- meta-llama/Meta-Llama-3-70B-Instruct
- meta-llama/Meta-Llama-3.1-405B-Instruct

## First-time Setup

On the first run, you'll be prompted to enter your API key. This key will be saved for future use.
(visit hyperbolic.xyz to get an API key with free credits)

## Workflow

1. Run the command with your desired file.
2. Enter a prompt describing the changes you want to make to the code.
3. The AI will process your request and suggest changes.
4. Review the proposed changes (displayed in a diff-like format).
5. Choose to apply or discard the changes.

## Examples

Process a JavaScript file:
`coders -f main.js`


Choose a model before processing a Python file:
`coders -f script.py -m`


## Note

Make sure you have a valid API key for the AI service. The tool will prompt you to enter it if it's not already saved.

### TODO:
[] - Add OpenRouter support
