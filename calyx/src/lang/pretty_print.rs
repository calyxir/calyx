use crate::lang::ast::*;
use atty::Stream;
use pretty::termcolor::{Color, ColorChoice, ColorSpec, StandardStream};
use pretty::RcDoc;
use std::io;

pub fn surround<'a, A>(
    pre: &'a str,
    doc: RcDoc<'a, A>,
    post: &'a str,
) -> RcDoc<'a, A> {
    RcDoc::text(pre).append(doc).append(RcDoc::text(post))
}

pub fn parens<A>(doc: RcDoc<A>) -> RcDoc<A> {
    surround("(", doc, ")")
}

pub fn brackets<A>(doc: RcDoc<A>) -> RcDoc<A> {
    surround("[", doc, "]")
}

fn define(doc: RcDoc<ColorSpec>) -> RcDoc<ColorSpec> {
    let mut c = ColorSpec::new();
    c.set_fg(Some(Color::Blue)).set_bold(true);
    doc.annotate(c)
}

fn port(doc: RcDoc<ColorSpec>) -> RcDoc<ColorSpec> {
    let mut c = ColorSpec::new();
    c.set_fg(Some(Color::Green));
    doc.annotate(c)
}

fn keyword(doc: RcDoc<ColorSpec>) -> RcDoc<ColorSpec> {
    let mut c = ColorSpec::new();
    c.set_fg(Some(Color::Blue));
    doc.annotate(c)
}

fn italic(doc: RcDoc<ColorSpec>) -> RcDoc<ColorSpec> {
    let mut c = ColorSpec::new();
    c.set_fg(Some(Color::Red));
    doc.annotate(c)
}

fn control(doc: RcDoc<ColorSpec>) -> RcDoc<ColorSpec> {
    let mut c = ColorSpec::new();
    c.set_fg(Some(Color::Green));
    doc.annotate(c)
}

fn enable(doc: RcDoc<ColorSpec>) -> RcDoc<ColorSpec> {
    let mut c = ColorSpec::new();
    c.set_fg(Some(Color::Yellow));
    doc.annotate(c)
}

fn small_vec<'a, T: PrettyPrint>(
    vec: &[T],
    arena: &'a bumpalo::Bump,
) -> RcDoc<'a, ColorSpec> {
    let docs = vec.iter().map(|s| s.prettify(&arena));
    RcDoc::intersperse(docs, RcDoc::space())
}

pub trait PrettyPrint {
    /// Convert `self` into an `RcDoc`. the `area` of type `&Bump`
    /// is provided in case objects need to be allocated while producing
    /// The RcDoc. Call `arena.alloc(obj)` to allocate `obj` and use the
    /// returned reference for printing.
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec>;
    fn pretty_print(&self) {
        // XXX(sam) this leaks memory atm because we put vecs into this
        let mut arena = bumpalo::Bump::new();
        {
            let str = self.prettify(&arena);
            if atty::is(Stream::Stdout) {
                str.render_colored(
                    100,
                    StandardStream::stdout(ColorChoice::Auto),
                )
                .unwrap();
            } else {
                str.render(100, &mut io::stdout()).unwrap();
            }
        }
        arena.reset();
    }
}

/* =============== Generic impls ================ */

impl PrettyPrint for String {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let r = arena.alloc(self.clone());
        RcDoc::text(&*r)
    }
}

impl PrettyPrint for &String {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let r = arena.alloc((*self).clone());
        RcDoc::text(&*r)
    }
}

impl PrettyPrint for u64 {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let r = arena.alloc(self.clone());
        RcDoc::text((*r).to_string())
    }
}

impl<T: PrettyPrint> PrettyPrint for Vec<T> {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let docs = self.iter().map(|s| s.prettify(&arena));
        RcDoc::intersperse(docs, RcDoc::line())
    }
}

impl<T: PrettyPrint, U: PrettyPrint> PrettyPrint for (T, U) {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let (t, u) = self;
        t.prettify(&arena)
            .append(RcDoc::space())
            .append(RcDoc::text("->"))
            .append(RcDoc::line())
            .append(u.prettify(&arena))
            .nest(2)
            .append(RcDoc::line())
    }
}

/* =============== Toplevel ================ */

impl PrettyPrint for NamespaceDef {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let comps = self.components.iter().map(|s| s.prettify(&arena));
        let inner = define(RcDoc::text("define/namespace"))
            .append(RcDoc::space())
            .append(italic(RcDoc::text(self.name.clone())))
            .append(RcDoc::line())
            .append(RcDoc::intersperse(
                comps,
                RcDoc::line().append(RcDoc::hardline()),
            ))
            .nest(2);
        parens(inner)
    }
}

impl PrettyPrint for ComponentDef {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let inner = define(RcDoc::text("define/component"))
            .append(RcDoc::space())
            .append(italic(RcDoc::text(self.name.clone())))
            .append(RcDoc::line())
            .append(
                parens(self.signature.inputs.prettify(&arena))
                    .nest(1)
                    .group(),
            )
            .append(RcDoc::line())
            .append(
                parens(self.signature.outputs.prettify(&arena))
                    .nest(1)
                    .group(),
            )
            .append(RcDoc::line())
            .append(parens(self.structure.prettify(&arena)).nest(1).group())
            .append(RcDoc::line())
            .append(self.control.prettify(&arena).group())
            .nest(2);
        parens(inner).append(RcDoc::line())
    }
}

impl PrettyPrint for Portdef {
    fn prettify<'a>(&self, _arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let inner = port(RcDoc::text("port"))
            .append(RcDoc::space())
            .append(RcDoc::text(self.name.clone()))
            .append(RcDoc::space())
            .append(RcDoc::text(self.width.to_string()));
        parens(inner)
    }
}

/* ============== Impls for Structure ================= */

impl PrettyPrint for Structure {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        match self {
            Structure::Decl { data } => data.prettify(&arena),
            Structure::Std { data } => data.prettify(&arena),
            Structure::Wire { data } => data.prettify(&arena),
        }
    }
}

impl PrettyPrint for Decl {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let inner = keyword(RcDoc::text("new"))
            .append(RcDoc::space())
            .append(italic(self.name.prettify(&arena)))
            .append(RcDoc::space())
            .append(self.component.prettify(&arena));
        brackets(inner)
    }
}

impl PrettyPrint for Std {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let inner = keyword(RcDoc::text("new-std"))
            .append(RcDoc::space())
            .append(italic(self.name.prettify(&arena)))
            .append(RcDoc::space())
            .append(self.instance.prettify(&arena))
            .group();
        brackets(inner)
    }
}

impl PrettyPrint for Wire {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let inner = RcDoc::text("->")
            .append(RcDoc::space())
            .append(self.src.prettify(&arena))
            .append(RcDoc::space())
            .append(self.dest.prettify(&arena));
        brackets(inner)
    }
}

impl PrettyPrint for Compinst {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        if self.params.is_empty() {
            parens(self.name.prettify(&arena))
        } else {
            let inner = self
                .name
                .prettify(&arena)
                .append(RcDoc::space())
                .append(self.params.prettify(&arena));
            parens(inner)
        }
    }
}

/* ============== Impls for Control ================= */

impl PrettyPrint for Control {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        match self {
            Control::Seq { data } => data.prettify(&arena),
            Control::Par { data } => data.prettify(&arena),
            Control::If { data } => data.prettify(&arena),
            Control::Ifen { data } => data.prettify(&arena),
            Control::While { data } => data.prettify(&arena),
            Control::Print { data } => data.prettify(&arena),
            Control::Enable { data } => data.prettify(&arena),
            Control::Disable { data } => data.prettify(&arena),
            Control::Empty { data } => data.prettify(&arena),
        }
    }
}

impl PrettyPrint for Seq {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let inner = control(RcDoc::text("seq"))
            .append(RcDoc::hardline())
            .append(self.stmts.prettify(&arena))
            .nest(1);
        parens(inner)
    }
}

impl PrettyPrint for Par {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let inner = control(RcDoc::text("par"))
            .append(RcDoc::hardline())
            .append(self.stmts.prettify(&arena))
            .nest(1);
        parens(inner)
    }
}

impl PrettyPrint for If {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let inner = control(RcDoc::text("if"))
            .append(RcDoc::space())
            .append(self.port.prettify(&arena))
            .append(RcDoc::space())
            .append(parens(small_vec(&self.cond, &arena)))
            .append(RcDoc::line())
            .append(self.tbranch.prettify(&arena))
            .append(RcDoc::line())
            .append(self.fbranch.prettify(&arena))
            .nest(1);
        parens(inner)
    }
}

impl PrettyPrint for Ifen {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let inner = control(RcDoc::text("ifen"))
            .append(RcDoc::space())
            .append(self.port.prettify(&arena))
            .append(RcDoc::space())
            .append(parens(small_vec(&self.cond, &arena)))
            .append(RcDoc::line())
            .append(self.tbranch.prettify(&arena))
            .append(RcDoc::line())
            .append(self.fbranch.prettify(&arena))
            .nest(1);
        parens(inner)
    }
}

impl PrettyPrint for While {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let inner = control(RcDoc::text("while"))
            .append(RcDoc::space())
            .append(self.port.prettify(&arena))
            .append(RcDoc::space())
            .append(parens(small_vec(&self.cond, &arena)))
            .append(RcDoc::line())
            .append(self.body.prettify(&arena))
            .nest(1);
        parens(inner)
    }
}

impl PrettyPrint for Print {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let inner = enable(RcDoc::text("print"))
            .append(RcDoc::line())
            .append(self.var.prettify(&arena))
            .group();
        parens(inner)
    }
}

impl PrettyPrint for Enable {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let inner = enable(RcDoc::text("enable"))
            .append(RcDoc::line())
            .append(self.comps.prettify(&arena))
            .group();
        parens(inner)
    }
}

impl PrettyPrint for Disable {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let inner = enable(RcDoc::text("disable"))
            .append(RcDoc::line())
            .append(self.comps.prettify(&arena))
            .group();
        parens(inner)
    }
}

impl PrettyPrint for Empty {
    fn prettify<'a>(&self, _arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let inner = enable(RcDoc::text("empty"));
        parens(inner)
    }
}

impl PrettyPrint for Port {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        match self {
            Port::Comp { component, port: p } => {
                let inner = port(RcDoc::text("@"))
                    .append(RcDoc::space())
                    .append(component.prettify(&arena))
                    .append(RcDoc::space())
                    .append(p.prettify(&arena));
                parens(inner)
            }
            Port::This { port: p } => {
                let inner = port(RcDoc::text("@"))
                    .append(RcDoc::space())
                    .append(keyword(RcDoc::text("this")))
                    .append(RcDoc::space())
                    .append(p.prettify(&arena));
                parens(inner)
            }
        }
    }
}
