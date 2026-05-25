mod cli;
mod commands;
mod config;
mod git;

use anyhow::Result;

fn main() -> Result<()> {
    match cli::parse() {
        cli::Invocation::Add { hook_name, args } => commands::add::run(&hook_name, &args)?,
        cli::Invocation::Run { hook_name, hook_args } => commands::run::run(&hook_name, &hook_args)?,
    }
    Ok(())
}
