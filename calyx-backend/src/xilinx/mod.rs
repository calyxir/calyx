//! Backend for generating synthesiable code for Xilinx FPGAs
mod axi;
mod axi_address_space;
mod control_axi;
mod fsm;
mod idl;
mod memory_axi;
mod toplevel;
mod utils;
mod xml;

pub use idl::IdlBackend;
pub use toplevel::XilinxInterfaceBackend;
pub use xml::XilinxXmlBackend;
