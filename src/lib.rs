use anyhow::Result;

pub mod app;
pub mod ui;
pub mod tui;
pub mod config;
pub mod api;
pub mod files;

/// Initialize the application
pub fn run() -> Result<()> {
    // This will be our main entry point for the application
    tui::run()
} 