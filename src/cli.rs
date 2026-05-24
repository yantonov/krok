use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "krok", version = clap::crate_version!(), about = "Git hook manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install krok as a git hook
    Install {
        /// Name of the git hook (e.g. pre-commit)
        hook_name: String,
    },
    /// Add a command to a hook's job list
    Add {
        /// Name of the git hook (e.g. pre-commit)
        hook_name: String,
        /// Command and arguments to register
        #[arg(trailing_var_arg = true, num_args = 1..)]
        args: Vec<String>,
    },
    /// Run jobs registered for a hook
    Run {
        /// Name of the git hook (e.g. pre-commit)
        hook_name: String,
        /// Arguments passed by git to the hook
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

pub enum Invocation {
    Install {
        hook_name: String,
    },
    Add {
        hook_name: String,
        args: Vec<String>,
    },
    Run {
        hook_name: String,
        hook_args: Vec<String>,
    },
}

pub fn parse() -> Invocation {
    match Cli::parse().command {
        Commands::Install { hook_name } => Invocation::Install { hook_name },
        Commands::Add { hook_name, args } => Invocation::Add { hook_name, args },
        Commands::Run { hook_name, args } => Invocation::Run { hook_name, hook_args: args },
    }
}
