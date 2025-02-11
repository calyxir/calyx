pub struct Measurement<'a> {
    name: &'a str,
    duration: std::time::Duration,
    start: std::time::Instant,
}

#[allow(unused)]
impl<'a> Measurement<'a> {
    pub fn new(name: &'a str) -> Self {
        Measurement {
            name,
            duration: std::time::Duration::new(0, 0),
            start: std::time::Instant::now(),
        }
    }

    pub fn start(&mut self) {
        self.start = std::time::Instant::now()
    }

    pub fn commit(&mut self) {
        self.duration += std::time::Instant::now() - self.start;
    }

    pub fn finalize(self, nest: usize) {
        eprintln!(
            "{}{} took {:?}",
            " ".repeat(nest * 2),
            self.name,
            self.duration
        );
    }
}
