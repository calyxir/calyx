use crate::lang::ast::*;
use pretty::RcDoc;

fn surround<'a>(pre: &'a str, doc: RcDoc<'a>, post: &'a str) -> RcDoc<'a> {
    RcDoc::text(pre).append(doc).append(RcDoc::text(post))
}

fn parens(doc: RcDoc) -> RcDoc {
    surround("(", doc, ")")
}

pub trait PrettyPrint {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a>;
    fn pretty_string(&self) -> String {
        let mut w = Vec::new();
        let arena = bumpalo::Bump::new();
        self.prettify(&arena).render(100, &mut w).unwrap();
        String::from_utf8(w).unwrap()
    }
    fn pretty_print(&self) {
        println!("{}", self.pretty_string());
    }
}

/* =============== Generic impls ================ */

impl PrettyPrint for String {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let r = arena.alloc(self.clone());
        RcDoc::text(&*r)
    }
}

impl PrettyPrint for &String {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let r = arena.alloc((*self).clone());
        RcDoc::text(&*r)
    }
}

impl PrettyPrint for u64 {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let r = arena.alloc(self.clone());
        RcDoc::text((*r).to_string())
    }
}

impl<T: PrettyPrint> PrettyPrint for Vec<T> {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let docs = self.iter().map(|s| s.prettify(&arena));
        RcDoc::intersperse(docs, RcDoc::line())
    }
}

impl<T: PrettyPrint, U: PrettyPrint> PrettyPrint for (T, U) {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let (t, u) = self;
        parens(
            t.prettify(&arena)
                .append(RcDoc::text(","))
                .append(RcDoc::space())
                .append(u.prettify(&arena)),
        )
    }
}

/* =============== Toplevel ================ */

impl PrettyPrint for NamespaceDef {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let comps = self.components.iter().map(|s| s.prettify(&arena));
        let inner = RcDoc::text("define/namespace")
            .append(RcDoc::space())
            .append(RcDoc::text(self.name.clone()))
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
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let inner = RcDoc::text("define/component")
            .append(RcDoc::space())
            .append(RcDoc::text(self.name.clone()))
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
        parens(inner)
    }
}

impl PrettyPrint for Portdef {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let inner = RcDoc::text("port")
            .append(RcDoc::space())
            .append(RcDoc::text(self.name.clone()))
            .append(RcDoc::space())
            .append(RcDoc::text(self.width.to_string()));
        parens(inner)
    }
}

/* ============== Impls for Structure ================= */

impl PrettyPrint for Structure {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        match self {
            Structure::Decl { data } => data.prettify(&arena),
            Structure::Std { data } => data.prettify(&arena),
            Structure::Wire { data } => data.prettify(&arena),
        }
    }
}

impl PrettyPrint for Decl {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let inner = RcDoc::text("new")
            .append(RcDoc::space())
            .append(self.name.prettify(&arena))
            .append(RcDoc::space())
            .append(self.component.prettify(&arena));
        parens(inner)
    }
}

impl PrettyPrint for Std {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let inner = RcDoc::text("new-std")
            .append(RcDoc::space())
            .append(self.name.prettify(&arena))
            .append(RcDoc::space())
            .append(self.instance.prettify(&arena))
            .group();
        parens(inner)
    }
}

impl PrettyPrint for Wire {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let inner = RcDoc::text("->")
            .append(RcDoc::space())
            .append(self.src.prettify(&arena))
            .append(RcDoc::space())
            .append(self.dest.prettify(&arena));
        parens(inner)
    }
}

impl PrettyPrint for Compinst {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
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
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
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
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let inner = RcDoc::text("seq")
            .append(RcDoc::hardline())
            .append(self.stmts.prettify(&arena))
            .nest(1);
        parens(inner)
    }
}

impl PrettyPrint for Par {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let inner = RcDoc::text("par")
            .append(RcDoc::hardline())
            .append(self.stmts.prettify(&arena))
            .nest(1);
        parens(inner)
    }
}

impl PrettyPrint for If {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let inner = RcDoc::text("if")
            .append(RcDoc::space())
            .append(self.cond.prettify(&arena))
            .append(RcDoc::line())
            .append(self.tbranch.prettify(&arena))
            .append(RcDoc::line())
            .append(self.fbranch.prettify(&arena))
            .nest(1);
        parens(inner)
    }
}

impl PrettyPrint for Ifen {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let inner = RcDoc::text("ifen")
            .append(RcDoc::space())
            .append(self.cond.prettify(&arena))
            .append(RcDoc::line())
            .append(self.tbranch.prettify(&arena))
            .append(RcDoc::line())
            .append(self.fbranch.prettify(&arena))
            .nest(1);
        parens(inner)
    }
}

impl PrettyPrint for While {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let inner = RcDoc::text("while")
            .append(RcDoc::space())
            .append(self.cond.prettify(&arena))
            .append(RcDoc::line())
            .append(self.body.prettify(&arena))
            .nest(1);
        parens(inner)
    }
}

impl PrettyPrint for Print {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let inner = RcDoc::text("print")
            .append(RcDoc::line())
            .append(self.var.prettify(&arena))
            .group();
        parens(inner)
    }
}

impl PrettyPrint for Enable {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let inner = RcDoc::text("enable")
            .append(RcDoc::line())
            .append(self.comps.prettify(&arena))
            .group();
        parens(inner)
    }
}

impl PrettyPrint for Disable {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let inner = RcDoc::text("enable")
            .append(RcDoc::line())
            .append(self.comps.prettify(&arena))
            .group();
        parens(inner)
    }
}

impl PrettyPrint for Empty {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        let inner = RcDoc::text("empty");
        parens(inner)
    }
}

impl PrettyPrint for Port {
    fn prettify<'a>(&self, arena: &'a bumpalo::Bump) -> RcDoc<'a> {
        match self {
            Port::Comp { component, port } => {
                let inner = RcDoc::text("@")
                    .append(RcDoc::space())
                    .append(component.prettify(&arena))
                    .append(RcDoc::space())
                    .append(port.prettify(&arena));
                parens(inner)
            }
            Port::This { port } => {
                let inner = RcDoc::text("@")
                    .append(RcDoc::space())
                    .append(RcDoc::text("this"))
                    .append(RcDoc::space())
                    .append(port.prettify(&arena));
                parens(inner)
            }
        }
    }
}
