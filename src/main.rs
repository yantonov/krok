mod cli;
mod commands;
mod config;
mod env;
mod git;
mod hooks;
mod logger;
mod shell;
mod wrapper;

use anyhow::Result;

use crate::env::Settings;
use crate::logger::StdLogger;

fn main() -> Result<()> {
    let settings = Settings::from_env();
    let logger = StdLogger::new(settings.verbose);
    match cli::parse() {
        cli::Commands::Add {
            hook_name,
            args,
            force,
        } => commands::add::run(&logger, &hook_name, &args, force)?,
        cli::Commands::Run { hook_name, args } => commands::run::run(&logger, &hook_name, &args)?,
        cli::Commands::Recover { hook_name, force } => {
            commands::recover::run(&logger, &hook_name, force)?
        }
        cli::Commands::Config { action } => match action {
            cli::ConfigAction::Show => commands::config::show(&logger)?,
            cli::ConfigAction::Edit => commands::config::edit(&logger)?,
            cli::ConfigAction::Path => commands::config::path(&logger)?,
        },
    }
    Ok(())
}
