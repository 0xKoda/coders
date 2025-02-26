use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();
    
    // Run the TUI application
    code_ai::run()
}