mod math;
mod mem_utils;
mod memories;

pub use math::*;

pub mod mem {
    use super::{
        mem_utils::{MemD1, MemD2, MemD3, MemD4},
        memories::{SeqMem, StdMem},
    };

    pub use super::memories::StdReg;

    pub type StdMemD1 = StdMem<MemD1>;
    pub type StdMemD2 = StdMem<MemD2>;
    pub type StdMemD3 = StdMem<MemD3>;
    pub type StdMemD4 = StdMem<MemD4>;

    pub type SeqMemD1 = SeqMem<MemD1>;
    pub type SeqMemD2 = SeqMem<MemD2>;
    pub type SeqMemD3 = SeqMem<MemD3>;
    pub type SeqMemD4 = SeqMem<MemD4>;
}
