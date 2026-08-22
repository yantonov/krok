use clap::{Parser, Subcommand};

// The commit is baked in by build.rs, so --version names the exact source the
// binary was built from without the version number alone having to be enough.
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")");

#[derive(Parser)]
#[command(name = "krok", version = VERSION, about = "Git hook manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a command to a hook's job list (installs the hook if needed)
    Add {
        /// Name of the git hook (e.g. pre-commit)
        hook_name: String,
        /// Command and arguments to register
        #[arg(trailing_var_arg = true, num_args = 1..)]
        args: Vec<String>,
        /// Skip validation that hook_name is a known git hook
        #[arg(short, long)]
        force: bool,
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
        /// Skip validation that hook_name is a known git hook
        #[arg(short, long)]
        force: bool,
    },
    /// Inspect or modify the krok config file
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Print the config file to stdout
    Show,
    /// Open the config file in the git editor
    Edit,
    /// Print the path to the config file
    Path,
}

pub fn parse() -> Commands {
    Cli::parse().command
}
