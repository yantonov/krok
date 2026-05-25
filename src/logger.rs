pub trait Logger {
    fn info(&self, msg: &str);
    fn error(&self, msg: &str);
}

pub struct StdLogger {
    verbose: bool,
}

impl StdLogger {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

impl Logger for StdLogger {
    fn info(&self, msg: &str) {
        if self.verbose {
            println!("{msg}");
        }
    }

    fn error(&self, msg: &str) {
        eprintln!("{msg}");
    }
}
