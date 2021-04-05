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

#[macro_export]
macro_rules! timed {
    ($name:expr, $body:expr) => {{
        eprint!("{}: ", $name);
        let start = std::time::Instant::now();
        let result = $body;
        let end = std::time::Instant::now();
        eprintln!("{:#?}", end - start);
        result
    }};
}

#[macro_export]
macro_rules! timed_nest {
    ($name:expr, $nest:expr, $body:expr) => {{
        eprintln!("{}{} start:", " ".repeat($nest * 2), $name);
        let start = std::time::Instant::now();
        let result = $body;
        let end = std::time::Instant::now();
        eprintln!("{}{} took {:#?}", " ".repeat($nest * 2), $name, end - start);
        result
    }};
    ($name:expr, $body:expr) => {
        timed_nest!($name, 1, $body)
    };
}

#[macro_export]
macro_rules! loop_timed {
    ($measurement:expr, $body:expr) => {{
        $measurement.start();
        let r = $body;
        $measurement.commit();
        r
    }};
}
