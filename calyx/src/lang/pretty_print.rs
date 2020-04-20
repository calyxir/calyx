use super::colors::ColorHelper;
use crate::lang::ast::*;
use atty::Stream;
use pretty::termcolor::{ColorChoice, ColorSpec, StandardStream};
use pretty::RcDoc;
use std::fmt;
use std::fmt::Display;
use std::io;
use std::io::Write;

pub trait PrettyHelper<'a>: Sized {
    fn surround(self, pre: &'a str, post: &'a str) -> Self;
    fn parens(self) -> Self {
        self.surround("(", ")")
    }
    fn brackets(self) -> Self {
        self.surround("[", "]")
    }
}

impl<'a, A> PrettyHelper<'a> for RcDoc<'a, A> {
    fn surround(self, l: &'a str, r: &'a str) -> Self {
        RcDoc::text(l).append(self).append(RcDoc::text(r))
    }
}

fn small_vec<'a, T: PrettyPrint>(
    vec: &[T],
    arena: &'a bumpalo::Bump,
) -> RcDoc<'a, ColorSpec> {
    let docs = vec.iter().map(|s| s.prettify(&arena));
    RcDoc::intersperse(docs, RcDoc::space())
}

pub fn display<W: Write>(doc: RcDoc<ColorSpec>, write: Option<W>) {
    if atty::is(Stream::Stdout) {
        doc.render_colored(100, StandardStream::stdout(ColorChoice::Auto))
            .unwrap();
    } else {
        match write {
            Some(mut w) => doc.render(100, &mut w).unwrap(),
            None => doc.render(100, &mut std::io::stdout()).unwrap(),
        }
    }
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

/* =============== Toplevel ================ */

impl PrettyPrint for Id {
    fn prettify<'a>(&self, _arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        RcDoc::text(self.to_string())
    }
}

impl PrettyPrint for NamespaceDef {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let comps = self.components.iter().map(|s| s.prettify(&arena));
        RcDoc::text("define/namespace")
            .define_color()
            .append(RcDoc::space())
            .append(self.name.prettify(&arena).ident_color())
            .append(RcDoc::line())
            .append(RcDoc::intersperse(
                comps,
                RcDoc::line().append(RcDoc::hardline()),
            ))
            .nest(2)
            .parens()
    }
}

impl PrettyPrint for ComponentDef {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        let inner = RcDoc::text("define/component")
            .define_color()
            .append(RcDoc::space())
            .append(self.name.prettify(&arena).ident_color())
            .append(RcDoc::line())
            .append(
                self.signature
                    .inputs
                    .prettify(&arena)
                    .parens()
                    .nest(1)
                    .group(),
            )
            .append(RcDoc::line())
            .append(
                self.signature
                    .outputs
                    .prettify(&arena)
                    .parens()
                    .nest(1)
                    .group(),
            )
            .append(RcDoc::line())
            .append(self.structure.prettify(&arena).nest(1).group().parens())
            .append(RcDoc::line())
            .append(self.control.prettify(&arena).group())
            .nest(2)
            .parens();
        inner.append(RcDoc::line())
    }
}

impl PrettyPrint for Portdef {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        RcDoc::text("port")
            .port_color()
            .append(RcDoc::space())
            .append(self.name.prettify(&arena))
            .append(RcDoc::space())
            .append(RcDoc::text(self.width.to_string()))
            .parens()
    }
}

/* ============== Impls for Structure ================= */

impl PrettyPrint for Structure {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        match self {
            Structure::Decl { data } => data.prettify(&arena),
            Structure::Std { data } => data.prettify(&arena),
            Structure::Wire { data } => data.prettify(&arena),
            Structure::Group { data } => data.prettify(&arena),
        }
    }
}

impl PrettyPrint for Decl {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        RcDoc::text("new")
            .keyword_color()
            .append(RcDoc::space())
            .append(self.name.prettify(&arena).ident_color())
            .append(RcDoc::space())
            .append(self.component.prettify(&arena))
            .brackets()
    }
}

impl PrettyPrint for Std {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        RcDoc::text("new-std")
            .keyword_color()
            .append(RcDoc::space())
            .append(self.name.prettify(&arena).ident_color())
            .append(RcDoc::space())
            .append(self.instance.prettify(&arena))
            .group()
            .brackets()
    }
}

impl PrettyPrint for Wire {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        RcDoc::text("->")
            .keyword_color()
            .append(RcDoc::space())
            .append(self.src.prettify(&arena))
            .append(RcDoc::space())
            .append(self.dest.prettify(&arena))
            .brackets()
    }
}

impl PrettyPrint for Group {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        RcDoc::text("group")
            .keyword_color()
            .append(RcDoc::space())
            .append(self.name.prettify(&arena).ident_color())
            .append(RcDoc::space())
            .append(self.comps.prettify(&arena).group().parens())
            .brackets()
    }
}

impl PrettyPrint for Compinst {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        if self.params.is_empty() {
            self.name.prettify(&arena).parens()
        } else {
            self.name
                .prettify(&arena)
                .append(RcDoc::space())
                .append(self.params.prettify(&arena))
                .parens()
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
            Control::While { data } => data.prettify(&arena),
            Control::Print { data } => data.prettify(&arena),
            Control::Enable { data } => data.prettify(&arena),
            Control::Empty { data } => data.prettify(&arena),
        }
    }
}

impl PrettyPrint for Seq {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        RcDoc::text("seq")
            .control_color()
            .append(RcDoc::hardline())
            .append(self.stmts.prettify(&arena))
            .nest(1)
            .parens()
    }
}

impl PrettyPrint for Par {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        RcDoc::text("par")
            .control_color()
            .append(RcDoc::hardline())
            .append(self.stmts.prettify(&arena))
            .nest(1)
            .parens()
    }
}

impl PrettyPrint for If {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        RcDoc::text("if")
            .control_color()
            .append(RcDoc::space())
            .append(self.port.prettify(&arena))
            .append(RcDoc::space())
            .append(small_vec(&self.cond, &arena).parens())
            .append(RcDoc::line())
            .append(self.tbranch.prettify(&arena))
            .append(RcDoc::line())
            .append(self.fbranch.prettify(&arena))
            .nest(1)
            .parens()
    }
}

impl PrettyPrint for While {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        RcDoc::text("while")
            .control_color()
            .append(RcDoc::space())
            .append(self.port.prettify(&arena))
            .append(RcDoc::space())
            .append(small_vec(&self.cond, &arena).parens())
            .append(RcDoc::line())
            .append(self.body.prettify(&arena))
            .nest(1)
            .parens()
    }
}

impl PrettyPrint for Print {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        RcDoc::text("print")
            .control_color()
            .append(RcDoc::line())
            .append(self.var.prettify(&arena))
            .group()
            .parens()
    }
}

impl PrettyPrint for Enable {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        RcDoc::text("enable")
            .enable_color()
            .append(RcDoc::space())
            .append(self.group.prettify(&arena))
            .group()
            .parens()
    }
}

impl PrettyPrint for Empty {
    fn prettify<'a>(&self, _arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        RcDoc::text("empty").control_color().parens()
    }
}

impl PrettyPrint for Port {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a, ColorSpec> {
        match self {
            Port::Comp { component, port: p } => RcDoc::text("@")
                .port_color()
                .append(RcDoc::space())
                .append(component.prettify(&arena))
                .append(RcDoc::space())
                .append(p.prettify(&arena))
                .parens(),
            Port::This { port: p } => RcDoc::text("@")
                .port_color()
                .append(RcDoc::space())
                .append(RcDoc::text("this").port_color())
                .append(RcDoc::space())
                .append(p.prettify(&arena))
                .parens(),
        }
    }
}

impl Display for Portdef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(port {} {})", self.name.to_string(), self.width)
    }
}
