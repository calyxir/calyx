use pretty::termcolor::{Color, ColorSpec};
use pretty::RcDoc;

pub trait ColorHelper {
    fn define_color(self) -> Self;
    fn port_color(self) -> Self;
    fn keyword_color(self) -> Self;
    fn ident_color(self) -> Self;
    fn control_color(self) -> Self;
    fn enable_color(self) -> Self;
}

impl<'a> ColorHelper for RcDoc<'a, ColorSpec> {
    fn define_color(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Blue)).set_bold(true);
        self.annotate(c)
    }

    fn port_color(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Green));
        self.annotate(c)
    }

    fn keyword_color(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Yellow));
        self.annotate(c)
    }

    fn ident_color(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Blue));
        self.annotate(c)
    }

    fn control_color(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Green));
        self.annotate(c)
    }

    fn enable_color(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Yellow));
        self.annotate(c)
    }
}

pub fn comment(doc: RcDoc<ColorSpec>) -> RcDoc<ColorSpec> {
    let mut c = ColorSpec::new();
    c.set_fg(Some(Color::Rgb(100, 100, 100)));
    doc.annotate(c)
}
