use crate::lang::ast::*;
use pretty::RcDoc;

fn surround<'a>(pre: &'a str, doc: RcDoc<'a>, post: &'a str) -> RcDoc<'a> {
    RcDoc::text(pre).append(doc).append(RcDoc::text(post))
}

fn parens(doc: RcDoc) -> RcDoc {
    surround("(", doc, ")")
}

pub trait PrettyPrint {
    fn prettify(&self) -> RcDoc;
    fn pretty_print(&self) {
        let mut w = Vec::new();
        self.prettify().render(80, &mut w).unwrap();
        println!("{}", String::from_utf8(w).unwrap());
    }
}

/* =============== Generic impls ================ */

impl PrettyPrint for String {
    fn prettify(&self) -> RcDoc {
        RcDoc::text(self)
    }
}

impl PrettyPrint for i64 {
    fn prettify(&self) -> RcDoc {
        RcDoc::text(self.to_string())
    }
}

impl<T: PrettyPrint> PrettyPrint for Vec<T> {
    fn prettify(&self) -> RcDoc {
        let docs = self.iter().map(|s| s.prettify());
        RcDoc::intersperse(docs, RcDoc::line())
    }
}

/* =============== Toplevel ================ */

impl PrettyPrint for Namespace {
    fn prettify(&self) -> RcDoc {
        let comps = self.components.iter().map(|s| s.prettify());
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

impl PrettyPrint for Component {
    fn prettify(&self) -> RcDoc {
        let inner = RcDoc::text("define/component")
            .append(RcDoc::space())
            .append(RcDoc::text(self.name.clone()))
            .append(RcDoc::line())
            .append(parens(self.inputs.prettify()).nest(1).group())
            .append(RcDoc::line())
            .append(parens(self.outputs.prettify()).nest(1).group())
            .append(RcDoc::line())
            .append(parens(self.structure.prettify()).nest(1).group())
            .append(RcDoc::line())
            .append(self.control.prettify().group())
            .nest(2);
        parens(inner)
    }
}

impl PrettyPrint for Portdef {
    fn prettify(&self) -> RcDoc {
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
    fn prettify(&self) -> RcDoc {
        match self {
            Structure::Decl { data } => data.prettify(),
            Structure::Std { data } => data.prettify(),
            Structure::Wire { data } => data.prettify(),
        }
    }
}

impl PrettyPrint for Decl {
    fn prettify(&self) -> RcDoc {
        let inner = RcDoc::text("new")
            .append(RcDoc::space())
            .append(self.name.prettify())
            .append(RcDoc::space())
            .append(self.component.prettify());
        parens(inner)
    }
}

impl PrettyPrint for Std {
    fn prettify(&self) -> RcDoc {
        let inner = RcDoc::text("new-std")
            .append(RcDoc::space())
            .append(self.name.prettify())
            .append(RcDoc::space())
            .append(self.instance.prettify())
            .group();
        parens(inner)
    }
}

impl PrettyPrint for Wire {
    fn prettify(&self) -> RcDoc {
        let inner = RcDoc::text("->")
            .append(RcDoc::space())
            .append(self.src.prettify())
            .append(RcDoc::space())
            .append(self.dest.prettify());
        parens(inner)
    }
}

impl PrettyPrint for Compinst {
    fn prettify(&self) -> RcDoc {
        if self.params.is_empty() {
            parens(self.name.prettify())
        } else {
            let inner = self
                .name
                .prettify()
                .append(RcDoc::space())
                .append(self.params.prettify());
            parens(inner)
        }
    }
}

/* ============== Impls for Control ================= */

impl PrettyPrint for Control {
    fn prettify(&self) -> RcDoc {
        match self {
            Control::Seq { data } => data.prettify(),
            Control::Par { data } => data.prettify(),
            Control::If { data } => data.prettify(),
            Control::Ifen { data } => data.prettify(),
            Control::While { data } => data.prettify(),
            Control::Print { data } => data.prettify(),
            Control::Enable { data } => data.prettify(),
            Control::Disable { data } => data.prettify(),
            Control::Empty { data } => data.prettify(),
        }
    }
}

impl PrettyPrint for Seq {
    fn prettify(&self) -> RcDoc {
        let inner = RcDoc::text("seq")
            .append(RcDoc::hardline())
            .append(self.stmts.prettify())
            .nest(1);
        parens(inner)
    }
}

impl PrettyPrint for Par {
    fn prettify(&self) -> RcDoc {
        let inner = RcDoc::text("par")
            .append(RcDoc::hardline())
            .append(self.stmts.prettify())
            .nest(1);
        parens(inner)
    }
}

impl PrettyPrint for If {
    fn prettify(&self) -> RcDoc {
        let inner = RcDoc::text("if")
            .append(RcDoc::space())
            .append(self.cond.prettify())
            .append(RcDoc::line())
            .append(self.tbranch.prettify())
            .append(RcDoc::line())
            .append(self.fbranch.prettify())
            .nest(1);
        parens(inner)
    }
}

impl PrettyPrint for Ifen {
    fn prettify(&self) -> RcDoc {
        let inner = RcDoc::text("ifen")
            .append(RcDoc::space())
            .append(self.cond.prettify())
            .append(RcDoc::line())
            .append(self.tbranch.prettify())
            .append(RcDoc::line())
            .append(self.fbranch.prettify())
            .nest(1);
        parens(inner)
    }
}

impl PrettyPrint for While {
    fn prettify(&self) -> RcDoc {
        let inner = RcDoc::text("while")
            .append(RcDoc::space())
            .append(self.cond.prettify())
            .append(RcDoc::line())
            .append(self.body.prettify())
            .nest(1);
        parens(inner)
    }
}

impl PrettyPrint for Print {
    fn prettify(&self) -> RcDoc {
        let inner = RcDoc::text("print")
            .append(RcDoc::space())
            .append(self.var.prettify());
        parens(inner)
    }
}

impl PrettyPrint for Enable {
    fn prettify(&self) -> RcDoc {
        let inner = RcDoc::text("enable")
            .append(RcDoc::space())
            .append(self.comps.prettify());
        parens(inner)
    }
}

impl PrettyPrint for Disable {
    fn prettify(&self) -> RcDoc {
        let inner = RcDoc::text("enable")
            .append(RcDoc::space())
            .append(self.comps.prettify());
        parens(inner)
    }
}

impl PrettyPrint for Empty {
    fn prettify(&self) -> RcDoc {
        let inner = RcDoc::text("empty");
        parens(inner)
    }
}

impl PrettyPrint for Port {
    fn prettify(&self) -> RcDoc {
        match self {
            Port::Comp { component, port } => {
                let inner = RcDoc::text("@")
                    .append(RcDoc::space())
                    .append(component.prettify())
                    .append(RcDoc::space())
                    .append(port.prettify());
                parens(inner)
            }
            Port::This { port } => {
                let inner = RcDoc::text("@")
                    .append(RcDoc::space())
                    .append(RcDoc::text("this"))
                    .append(RcDoc::space())
                    .append(port.prettify());
                parens(inner)
            }
        }
    }
}
