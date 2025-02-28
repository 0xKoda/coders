#![warn(clippy::all)]

use anyhow::Result;
use std::fs;
use log::{info, LevelFilter};
use log4rs::{
    append::{
        file::FileAppender,
    },
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};

fn setup_logging() -> Result<()> {
    // Ensure the logs directory exists
    let log_dir = "logs";
    fs::create_dir_all(log_dir)?;
    
    // Create a file appender
    let file_appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S)} [{l}] - {t} - {m}{n}")))
        .build(format!("{}/code_ai.log", log_dir))?;
    
    // Configure the logging system - file only, no console output
    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(file_appender)))
        .build(Root::builder()
            .appender("file")
            .build(LevelFilter::Debug))?;
    
    // Initialize the logging system
    log4rs::init_config(config)?;
    
    info!("Logging initialized");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    setup_logging()?;
    
    info!("Starting Code-AI Assistant");
    
    // Run the TUI application
    match code_ai::run() {
        Ok(_) => {
            info!("Application exited normally");
            Ok(())
        }
        Err(e) => {
            log::error!("Application error: {}", e);
            Err(e)
        }
    }
}