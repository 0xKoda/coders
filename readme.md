# Coders AI Assistant

Coders is an AI-powered command-line tool that helps you create, modify and improve your code.

## Features
- Ceate complete files, patch, find and fix bugs, and much more
- Diff view output: view changes in the terminal before accepting
- Hybrid Model selection: choose between various models from different providers
- Quickly iterate on code and run within the terminal.


## Options

- `-f, --file <FILE>`: Specify the file to process (required)
- `-m, --model`: Enable model selection
- `-h, --help`: Display help information and all available options
- `-V, --version`: Print version information

## Available Models
[OpenRouter]
- nousresearch/hermes-3-llama-3.1-405b
- nousresearch/hermes-3-llama-3.1-405b:extended
- meta-llama/llama-3.1-8b-instruct:free

[Hyperbolic]
- NousResearch/Hermes-3-Llama-3.1-70B
- meta-llama/Meta-Llama-3.1-70B-Instruct
- meta-llama/Meta-Llama-3.1-8B-Instruct
- meta-llama/Meta-Llama-3-70B-Instruct
- meta-llama/Meta-Llama-3.1-405B-Instruct

### Best model for code editing
- nousresearch/hermes-3-llama-3.1-405b:extended

## First-time Setup

On the first run, you'll be prompted to enter your API key. This key will be saved for future use. Hyperbolic provides free signup credits. Openrouter provides nousresearch hermes-3-llama-3.1-405b for free currently.

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
`coders -f -m script.py `


Choose 'openrouter' models
`coders -o -f script.py`

## Note

Make sure you have a valid API key. The tool will prompt you to enter it if it's not already saved.