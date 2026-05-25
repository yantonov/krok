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
