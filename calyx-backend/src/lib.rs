//! Backends for the Calyx compiler.
mod backend_opt;
mod traits;
mod verilog;

pub use backend_opt::BackendOpt;
pub use traits::Backend;
pub use verilog::VerilogBackend;

#[cfg(feature = "mlir")]
mod mlir;
#[cfg(feature = "mlir")]
pub use mlir::MlirBackend;

#[cfg(feature = "resources")]
mod resources;
#[cfg(feature = "resources")]
pub use resources::ResourcesBackend;

#[cfg(feature = "sexp")]
mod sexp;
#[cfg(feature = "sexp")]
pub use sexp::SexpBackend;

#[cfg(feature = "xilinx")]
pub mod xilinx;
