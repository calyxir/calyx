/// A simple macro which unwraps a Result. If the result is an error, it will
/// print the error and continue. Otherwise it returns the value.
macro_rules! unwrap_error_message {
    ($name:ident) => {
        let $name = match $name {
            Ok(v) => v,
            Err(e) => {
                println!("Error: {}", owo_colors::OwoColorize::red(&e));
                continue;
            }
        };
    };
}
pub(crate) use unwrap_error_message;
