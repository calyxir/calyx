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
    fn stylize_assignment<'a>(
        &'a self,
    ) -> owo_colors::SupportsColorsDisplay<
        '_,
        Self,
        owo_colors::Styled<&Self>,
        impl Fn(&'a Self) -> owo_colors::Styled<&Self>,
    > {
        self.if_supports_color(Stdout, |text| text.style(ASSIGN_STYLE))
    }

    fn stylize_usage_example<'a>(
        &'a self,
    ) -> owo_colors::SupportsColorsDisplay<
        '_,
        Self,
        owo_colors::Styled<&Self>,
        impl Fn(&'a Self) -> owo_colors::Styled<&Self>,
    > {
        // this cannot be a const due to reasons
        let style = Style::new().blue().italic();
        self.if_supports_color(Stdout, move |text| text.style(style))
    }

    fn stylize_name<'a>(
        &'a self,
    ) -> owo_colors::SupportsColorsDisplay<
        '_,
        Self,
        owo_colors::styles::UnderlineDisplay<'_, Self>,
        impl Fn(&'a Self) -> owo_colors::styles::UnderlineDisplay<'_, Self>,
    > {
        self.if_supports_color(Stdout, move |text| text.underline())
    }

    fn stylize_error<'a>(
        &'a self,
    ) -> owo_colors::SupportsColorsDisplay<
        '_,
        Self,
        owo_colors::FgColorDisplay<'_, owo_colors::colors::Red, Self>,
        impl Fn(
            &'a Self,
        )
            -> owo_colors::FgColorDisplay<'_, owo_colors::colors::Red, Self>,
    > {
        self.if_supports_color(Stdout, |text| text.red())
    }

    fn stylize_debugger_missing(&self) -> Box<dyn Display + '_> {
        let style = Style::new().red().bold().strikethrough();
        Box::new(self.if_supports_color(Stdout, move |text| text.style(style)))
    }
}

impl<T: OwoColorize + Display> Color for T {}
