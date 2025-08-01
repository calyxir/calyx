use itertools::Itertools;
use std::str::FromStr;

/// Enumeration of valid backends
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum BackendOpt {
    #[default]
    Calyx,
    Verilog,
    Xilinx,
    XilinxXml,
    Mlir,
    Resources,
    Sexp,
    Firrtl,
    PrimitiveUses,
    None,
}

/// Return a vector that maps strings to Backends.
#[inline(always)]
fn backends() -> Vec<(&'static str, BackendOpt)> {
    vec![
        ("verilog", BackendOpt::Verilog),
        ("xilinx", BackendOpt::Xilinx),
        ("xilinx-xml", BackendOpt::XilinxXml),
        ("calyx", BackendOpt::Calyx),
        ("mlir", BackendOpt::Mlir),
        ("resources", BackendOpt::Resources),
        ("sexp", BackendOpt::Sexp),
        ("firrtl", BackendOpt::Firrtl),
        ("primitive-uses", BackendOpt::PrimitiveUses),
        ("none", BackendOpt::None),
    ]
}

/// Command line parsing for the Backend enum
impl FromStr for BackendOpt {
    type Err = String;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        // allocate a vector for the list of backends
        let backends = backends();
        // see if there is a backend for the string that we receive
        let found_backend = backends
            .iter()
            .find(|(backend_name, _)| &input == backend_name);
        if let Some((_, opt)) = found_backend {
            // return the BackendOpt if we found one
            Ok(opt.clone())
        } else {
            // build list of backends for error message
            let backend_str = backends
                .iter()
                .map(|(name, _)| (*name).to_string())
                .join(", ");
            Err(format!(
                "`{input}` is not a valid backend.\nValid backends: {backend_str}"
            ))
        }
    }
}

/// Convert `BackendOpt` to a string
impl std::fmt::Display for BackendOpt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mlir => "mlir",
            Self::Resources => "resources",
            Self::Sexp => "sexp",
            Self::Verilog => "verilog",
            Self::Xilinx => "xilinx",
            Self::XilinxXml => "xilinx-xml",
            Self::Calyx => "calyx",
            Self::Firrtl => "firrtl",
            Self::PrimitiveUses => "primitive-uses",
            Self::None => "none",
        }
        .fmt(f)
    }
}
