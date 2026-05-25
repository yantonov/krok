mod cli;
mod commands;
mod config;
mod env;
mod git;
mod logger;

use anyhow::Result;

use crate::env::Settings;
use crate::logger::StdLogger;

fn main() -> Result<()> {
    let settings = Settings::from_env();
    let logger = StdLogger::new(settings.verbose);
    match cli::parse() {
        cli::Invocation::Add { hook_name, args } => {
            commands::add::run(&logger, &hook_name, &args)?
        }
        cli::Invocation::Run { hook_name, hook_args } => {
            commands::run::run(&logger, &hook_name, &hook_args)?
        }
    }
    Ok(())
}
