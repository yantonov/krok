use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "krok", version = clap::crate_version!(), about = "Git hook manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a command to a hook's job list (installs the hook if needed)
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
    /// Restore the wrapper script for a hook that has been replaced or removed
    Recover {
        /// Name of the git hook (e.g. pre-commit)
        hook_name: String,
    },
    /// Inspect or modify the krok config file
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Print the config file to stdout
    Show,
    /// Open the config file in the git editor
    Edit,
}

pub enum Invocation {
    Add {
        hook_name: String,
        args: Vec<String>,
    },
    Run {
        hook_name: String,
        hook_args: Vec<String>,
    },
    Recover {
        hook_name: String,
    },
    ConfigShow,
    ConfigEdit,
}

pub fn parse() -> Invocation {
    match Cli::parse().command {
        Commands::Add { hook_name, args } => Invocation::Add { hook_name, args },
        Commands::Run { hook_name, args } => Invocation::Run { hook_name, hook_args: args },
        Commands::Recover { hook_name } => Invocation::Recover { hook_name },
        Commands::Config { action } => match action {
            ConfigAction::Show => Invocation::ConfigShow,
            ConfigAction::Edit => Invocation::ConfigEdit,
        },
    }
}
