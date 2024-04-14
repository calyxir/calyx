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
    #[cfg(feature = "yxi")]
    Yxi,
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
        #[cfg(feature = "yxi")]
        ("yxi", BackendOpt::Yxi),
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
                "`{}` is not a valid backend.\nValid backends: {}",
                input, backend_str
            ))
        }
    }
}

/// Convert `BackendOpt` to a string
impl ToString for BackendOpt {
    fn to_string(&self) -> String {
        match self {
            Self::Mlir => "mlir",
            Self::Resources => "resources",
            Self::Sexp => "sexp",
            Self::Verilog => "verilog",
            Self::Xilinx => "xilinx",
            Self::XilinxXml => "xilinx-xml",
            #[cfg(feature = "yxi")]
            Self::Yxi => "yxi",
            Self::Calyx => "calyx",
            Self::Firrtl => "firrtl",
            Self::PrimitiveUses => "primitive-uses",
            Self::None => "none",
        }
        .to_string()
    }
}
