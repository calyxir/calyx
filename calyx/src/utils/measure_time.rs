#[macro_export]
macro_rules! timed {
    ($name:expr, $body:expr) => {{
        eprint!("measuring {}: ", $name);
        let start = std::time::Instant::now();
        let result = $body;
        let end = std::time::Instant::now();
        eprintln!("{:#?}", end - start);
        result
    }};
}
