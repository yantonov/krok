pub trait Logger {
    fn debug(&self, msg: &str);
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
    fn debug(&self, msg: &str) {
        if self.verbose {
            println!("{msg}");
        }
    }

    fn info(&self, msg: &str) {
        println!("{msg}");
    }

    fn error(&self, msg: &str) {
        eprintln!("{msg}");
    }
}
