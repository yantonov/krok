pub trait Logger {
    fn info(&self, msg: &str);
    fn error(&self, msg: &str);
}

pub struct StdLogger;

impl Logger for StdLogger {
    fn info(&self, msg: &str) {
        println!("{msg}");
    }

    fn error(&self, msg: &str) {
        eprintln!("{msg}");
    }
}
