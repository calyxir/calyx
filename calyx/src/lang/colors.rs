use pretty::termcolor::{Color, ColorSpec};
use pretty::RcDoc;

pub trait ColorHelper {
    fn define(self) -> Self;
    fn port(self) -> Self;
    fn keyword(self) -> Self;
    fn ident(self) -> Self;
    fn control(self) -> Self;
    fn enable(self) -> Self;
}

impl<'a> ColorHelper for RcDoc<'a, ColorSpec> {
    fn define(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Blue)).set_bold(true);
        self.annotate(c)
    }

    fn port(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Green));
        self.annotate(c)
    }

    fn keyword(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Yellow));
        self.annotate(c)
    }

    fn ident(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Blue));
        self.annotate(c)
    }

    fn control(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Green));
        self.annotate(c)
    }

    fn enable(self) -> Self {
        let mut c = ColorSpec::new();
        c.set_fg(Some(Color::Yellow));
        self.annotate(c)
    }
}
