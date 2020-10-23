//! Defines common colors schemes for FuTIL syntax.
use pretty::termcolor::{Color, ColorSpec};
use pretty::RcDoc;

/// Colors for various constructs in FuTIL
pub trait ColorHelper {
    /// Color for definition commands
    fn define_color(self) -> Self;
    /// Color for generic keywords
    fn keyword_color(self) -> Self;
    /// Color for control statements
    fn control_color(self) -> Self;
    /// Color for literals
    fn literal_color(self) -> Self;
}

impl<'a> ColorHelper for RcDoc<'a, ColorSpec> {
    fn define_color(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Green)).set_bold(true);
        self.annotate(c)
    }

    fn keyword_color(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Blue));
        self.annotate(c)
    }

    fn control_color(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Yellow));
        self.annotate(c)
    }

    fn literal_color(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Magenta));
        self.annotate(c)
    }
}

/// Comment out a given document that colorize it.
pub fn comment(doc: RcDoc<ColorSpec>) -> RcDoc<ColorSpec> {
    let mut c = ColorSpec::new();
    c.set_fg(Some(Color::Rgb(100, 100, 100)));
    doc.annotate(c)
}
