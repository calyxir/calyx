//! Inlcude all of the base primitives defined by the Calyx compiler.

/// A macro that defines an array that maps the file paths to the contents.
/// Usage:
/// ```
/// load_prims! { CORE, "core.futil", "core.sv" }
/// ```
/// This will define the variables `CORE_FUTIL` and `CORE_SV` and the array
/// `CORE`.
macro_rules! load_prims {
    ($name:ident, $futil_path:literal, $sv_path:literal) => {
        pub const $name: [(&str, &str); 2] = [
            (
                $futil_path,
                include_str!(concat!("../primitives/", $futil_path)),
            ),
            ($sv_path, include_str!(concat!("../primitives/", $sv_path))),
        ];
    };
}

load_prims! { CORE, "core.futil", "core.sv" }
load_prims! { BINARY_OPERATORS, "binary_operators.futil", "binary_operators.sv" }
load_prims! { MATH, "math.futil", "math.sv" }
load_prims! { MEMORIES, "memories.futil", "memories.sv" }
load_prims! { PIPELINED, "pipelined.futil", "pipelined.sv" }
load_prims! { SYNC, "sync.futil", "sync.sv" }

/// The core primitive in the compiler
pub const COMPILE_LIB: (&str, &str) =
    ("compile.futil", include_str!("../primitives/compile.futil"));

pub const KNOWN_LIBS: [(&str, [(&str, &str); 2]); 6] = [
    ("core", CORE),
    ("binary_operators", BINARY_OPERATORS),
    ("math", MATH),
    ("memories", MEMORIES),
    ("pipelined", PIPELINED),
    ("sync", SYNC),
];
