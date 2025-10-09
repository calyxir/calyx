use std::{
    fmt::{Display, Write},
    path::Path,
};

use owo_colors::{OwoColorize, Stream::Stdout, Style};

use crate::configuration::ColorConfig;

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

pub const ASSIGN_STYLE: Style = Style::new().yellow();

/// A trait to standardize the coloring throughout Cider while still respecting the
/// color setting. It is automatically implemented for most relevant types.
pub trait Color: OwoColorize + Display {
    fn stylize_assignment(&self) -> impl Display {
        self.if_supports_color(Stdout, |text| text.style(ASSIGN_STYLE))
    }

    fn stylize_usage_example(&self) -> impl Display {
        // this cannot be a const due to reasons
        let style = Style::new().blue().italic();
        self.if_supports_color(Stdout, move |text| text.style(style))
    }

    fn stylize_name(&self) -> impl Display {
        self.if_supports_color(Stdout, move |text| text.underline())
    }

    fn stylize_error(&self) -> impl Display {
        let style = Style::new().red().bold();
        self.if_supports_color(Stdout, move |text| text.style(style))
    }

    fn stylize_debugger_missing(&self) -> impl Display {
        let style = Style::new().red().bold().strikethrough();
        Box::new(self.if_supports_color(Stdout, move |text| text.style(style)))
    }

    fn stylize_print_code(&self) -> impl Display {
        let style = Style::new().cyan().underline();
        Box::new(self.if_supports_color(Stdout, move |text| text.style(style)))
    }

    fn stylize_warning(&self) -> impl Display {
        let style = Style::new().yellow().italic();
        Box::new(self.if_supports_color(Stdout, move |text| text.style(style)))
    }

    fn stylize_command(&self) -> impl Display {
        let style = Style::new().yellow().bold().underline();
        Box::new(self.if_supports_color(Stdout, move |text| text.style(style)))
    }

    fn stylize_breakpoint(&self) -> impl Display {
        let style = Style::new().yellow().underline();
        Box::new(self.if_supports_color(Stdout, move |text| text.style(style)))
    }

    fn stylize_command_description(&self) -> impl Display {
        let style = Style::new().yellow();
        Box::new(self.if_supports_color(Stdout, move |text| text.style(style)))
    }

    fn stylize_breakpoint_enabled(&self) -> impl Display {
        let style = Style::new().green();
        Box::new(self.if_supports_color(Stdout, move |text| text.style(style)))
    }

    fn stylize_breakpoint_disabled(&self) -> impl Display {
        let style = Style::new().red();
        Box::new(self.if_supports_color(Stdout, move |text| text.style(style)))
    }

    fn stylize_value(&self) -> impl Display {
        let style = Style::new().bold();
        Box::new(self.if_supports_color(Stdout, move |text| text.style(style)))
    }

    fn stylize_port_name(&self) -> impl Display {
        let style = Style::new().yellow();
        Box::new(self.if_supports_color(Stdout, move |text| text.style(style)))
    }

    fn stylize_underline(&self) -> impl Display {
        let style = Style::new().underline();
        Box::new(self.if_supports_color(Stdout, move |text| text.style(style)))
    }

    fn stylize_bold(&self) -> impl Display {
        let style = Style::new().bold();
        Box::new(self.if_supports_color(Stdout, move |text| text.style(style)))
    }
}

impl<T: OwoColorize + Display> Color for T {}

pub fn print_debugger_welcome() {
    println!(
        "==== {}: The {}alyx {}nterpreter and {}bugge{} ====",
        "Cider".stylize_bold(),
        "C".stylize_underline(),
        "I".stylize_underline(),
        "De".stylize_underline(),
        "r".stylize_underline()
    );
}

pub(crate) fn force_color(force_color: ColorConfig) {
    match force_color {
        ColorConfig::On => owo_colors::set_override(true),
        ColorConfig::Off => owo_colors::set_override(false),
        ColorConfig::Auto => owo_colors::unset_override(),
    }
}

pub fn format_file_line(
    line_num: usize,
    line_content: String,
    file_path: &Path,
) -> String {
    format!(
        "({}: {line_num}) {line_content}",
        file_path.to_string_lossy()
    )
}
