use std::fmt::{Display, Write};

use owo_colors::{OwoColorize, Stream::Stdout, Style};

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
}

impl<T: OwoColorize + Display> Color for T {}

pub fn print_debugger_welcome() {
    println!(
        "==== {}: The {}alyx {}nterpreter and {}bugge{} ====",
        "Cider".bold(),
        "C".underline(),
        "I".underline(),
        "De".underline(),
        "r".underline()
    );
}

pub(crate) fn force_color(force_color: bool) {
    if force_color {
        owo_colors::set_override(true);
    }
    // Allow inference of color if not forced rather than forcing no colors
    else {
        owo_colors::unset_override();
    }
}
