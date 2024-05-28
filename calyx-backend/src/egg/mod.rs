//! Backend for generating synthesiable code for Xilinx FPGAs
mod calyx_to_egg;
mod cost_model;
mod egg_conversion;
mod egg_optimize;
mod egg_to_calyx;
mod unit_tests;
mod utils;

pub use egg_conversion::EggBackend;
pub use egg_optimize::EggOptimizeBackend;
