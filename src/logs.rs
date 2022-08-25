struct LogScraper {}

impl LogScraper {
    pub fn process_line(&self, line: &str) {
        if line.contains("Pause countdown done") || line.contains("Got rewards") {}
    }
}
