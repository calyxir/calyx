//! Backends for the Calyx compiler.
mod backend_opt;
mod firrtl;
mod primitive_uses;
mod traits;
mod verilog;
mod yxi;

pub use backend_opt::BackendOpt;
pub use firrtl::FirrtlBackend;
pub use primitive_uses::PrimitiveUsesBackend;
pub use traits::Backend;
pub use verilog::VerilogBackend;
pub use yxi::YxiBackend;

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
