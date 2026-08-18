/// Exported to every job: the top level of the working tree.
pub const REPO_ROOT_VAR: &str = "KROK_REPO_ROOT";

/// Exported to every job: the directory hook scripts live in.
pub const HOOKS_DIR_VAR: &str = "KROK_HOOKS_DIR";

/// Exported to every job: the git directory shared by every worktree.
///
/// Unlike the hooks directory, nothing outside git moves this one, which is what
/// state krok wrote earlier has to be found through.
pub const GIT_DIR_VAR: &str = "KROK_GIT_DIR";

pub struct Settings {
    pub verbose: bool,
}

impl Settings {
    pub fn from_env() -> Self {
        Self {
            verbose: read_bool_var("KROK_DEBUG"),
        }
    }
}

fn read_bool_var(name: &str) -> bool {
    match std::env::var(name) {
        Ok(value) => {
            let normalized = value.trim().to_ascii_lowercase();
            normalized == "1" || normalized == "true"
        }
        Err(_) => false,
    }
}
