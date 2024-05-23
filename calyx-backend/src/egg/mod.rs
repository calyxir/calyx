//! Backend for generating synthesiable code for Xilinx FPGAs
mod calyx_to_egg;
mod egg;
mod egg_optimize;
mod egg_to_calyx;
mod unit_tests;
mod utils;

pub use egg::EggBackend;
pub use egg_optimize::EggOptimizeBackend;
