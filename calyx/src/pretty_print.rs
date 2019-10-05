use crate::ast::*;
use crate::utils::*;

/**
 * Conversion function from identifiers to string
 */
pub fn id_to_string(id: Id) -> String {
    return id;
}

pub trait Printable {
    fn prettify( &self ) -> String;
    fn pretty_print( &self ) -> () {
        println!("{}", self.prettify());
    }
}

impl Printable for Namespace {
    fn prettify(&self) -> String {
        let comps = self.components.iter().map(|c| c.prettify()).collect();
        return format!("( define/namespace {} {} )", self.name, combine(&comps, " ",""));
    }
}

impl Printable for Component {
    fn prettify(&self) -> String {
        return format!("( define/component {} {} {} {} {} )",
            self.name,
            combine(&self.inputs.iter().map(|i| i.prettify()).collect(), "\r\n", ""),
            combine(&self.outputs.iter().map(|o| o.prettify()).collect(), "\r\n", ""),
            combine(&self.structure.iter().map(|s| s.prettify()).collect(), "\r\n", ""),
            self.control.prettify()
        );
    }
}

impl Printable for Control {
    // TODO handle nested levels
    fn prettify( &self ) -> String {
        match self {
            Control::Seq(vec) => {
                let seq = vec.iter().map(|c| c.prettify()).collect();
                return format!("( seq\r\n    {})", combine(&seq, "\r\n    ","\r\n"));
            },
            Control::Par(vec) => {
                let par = vec.iter().map(|c| c.prettify()).collect();
                return format!("( par\r\n    {})", combine(&par, "\r\n    ","\r\n"));
            },
            Control::If{ cond, tbranch, fbranch } => {
                return format!("( if {}\r\n    {}\r\n    {}\r\n)", cond.prettify(), tbranch.prettify(), fbranch.prettify());
            },
            Control::Ifen{ cond, tbranch, fbranch } => {
                return format!("( ifen {}\r\n    {}\r\n    {}\r\n)", cond.prettify(), tbranch.prettify(), fbranch.prettify());
            },
            Control::While{ cond, body } => {
                return format!("( while {}\r\n    {}\r\n)", cond.prettify(), body.prettify());
            }
            Control::Print(id) => {
                return format!("( print\r\n    {}\r\n)", id.to_string());
            }
            Control::Enable(vec) => {
                return format!("( enable {} )", combine(vec, " ",""));
            }
            Control::Disable(vec) => {
                return format!("( disable {} )", combine(vec, " ",""));
            }
            Control::Empty => {
                return format!("( empty )");
            }
        }
    }
}

impl Printable for Structure {
    fn prettify( &self ) -> String {
        match self {
            Structure::Decl{ name, component } => return format!("( new {} {} )", name, component),
            Structure::Std{ name, instance } => return format!("( new-std {} {} )", name, instance.prettify()),
            Structure::Wire{ src, dest } => return format!("( -> {} {} )", src.prettify(), dest.prettify()),
        }
    }
}

impl Printable for Portdef {
    fn prettify( &self ) -> String {
        return format!("( port {} {} )", id_to_string(self.name.clone()), self.width.to_string());
    }
}

impl Printable for Port {
    fn prettify( &self ) -> String {
        match self {
            Port::Comp{ component, port } => return format!("( @ {} {} )", component, port),
            Port::This{ port } => return format!("( @ this {} )", port),
        }
    }
}

impl Printable for Compinst {
    fn prettify( &self ) -> String {
        let params = self.params.iter().map(|p| p.to_string()).collect();
        return format!("( {} {} )", self.name, combine(&params, " ",""));
    }
}