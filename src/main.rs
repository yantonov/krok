mod cli;
mod commands;
mod config;
mod env;
mod git;
mod hooks;
mod logger;
mod wrapper;

use anyhow::Result;

use crate::env::Settings;
use crate::logger::StdLogger;

fn main() -> Result<()> {
    let settings = Settings::from_env();
    let logger = StdLogger::new(settings.verbose);
    match cli::parse() {
        cli::Invocation::Add { hook_name, args, force } => {
            commands::add::run(&logger, &hook_name, &args, force)?
        }
        cli::Invocation::Run { hook_name, hook_args } => {
            commands::run::run(&logger, &hook_name, &hook_args)?
        }
        cli::Invocation::Recover { hook_name, force } => {
            commands::recover::run(&logger, &hook_name, force)?
        }
        cli::Invocation::ConfigShow => commands::config::show(&logger)?,
        cli::Invocation::ConfigEdit => commands::config::edit(&logger)?,
        cli::Invocation::ConfigPath => commands::config::path(&logger)?,
    }
    Ok(())
}
