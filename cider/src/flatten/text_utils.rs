use std::fmt::Write;

pub const INDENTATION: &str = "    ";

/// Indents each line in the given string by the indentation count.
pub fn indent<S: AsRef<str>>(target: S, indent_count: usize) -> String {
    let mut out = String::new();

    let mut first_flag = true;

    for line in target.as_ref().lines() {
        if first_flag {
            first_flag = false;
        } else {
            writeln!(out).unwrap();
        }

        if !line.is_empty() {
            write!(out, "{}{}", INDENTATION.repeat(indent_count), line)
                .unwrap();
        }
    }

    if target.as_ref().ends_with('\n') {
        writeln!(out).unwrap();
    }

    out
}
