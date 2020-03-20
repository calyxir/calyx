use pretty::termcolor::{Color, ColorSpec};
use pretty::RcDoc;

pub fn define(doc: RcDoc<ColorSpec>) -> RcDoc<ColorSpec> {
    let mut c = ColorSpec::new();
    c.set_fg(Some(Color::Blue)).set_bold(true);
    doc.annotate(c)
}

pub fn port(doc: RcDoc<ColorSpec>) -> RcDoc<ColorSpec> {
    let mut c = ColorSpec::new();
    c.set_fg(Some(Color::Green));
    doc.annotate(c)
}

pub fn keyword(doc: RcDoc<ColorSpec>) -> RcDoc<ColorSpec> {
    let mut c = ColorSpec::new();
    c.set_fg(Some(Color::Blue));
    doc.annotate(c)
}

pub fn ident(doc: RcDoc<ColorSpec>) -> RcDoc<ColorSpec> {
    let mut c = ColorSpec::new();
    c.set_fg(Some(Color::Red));
    doc.annotate(c)
}

pub fn control(doc: RcDoc<ColorSpec>) -> RcDoc<ColorSpec> {
    let mut c = ColorSpec::new();
    c.set_fg(Some(Color::Green));
    doc.annotate(c)
}

pub fn enable(doc: RcDoc<ColorSpec>) -> RcDoc<ColorSpec> {
    let mut c = ColorSpec::new();
    c.set_fg(Some(Color::Yellow));
    doc.annotate(c)
}
