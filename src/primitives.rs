//! Inlcude all of the base primitives defined by the Calyx compiler.

/// Calyx files that are known primitive to the compiler. Contains both the
/// Calyx definition file and the SystemVerilog file.
pub const KNOWN_PRIMITIVES: [(&str, &str); 6] = [
    (
        "primitives/binary_operators.futil",
        "primitives/binary_operators.sv",
    ),
    ("primitives/core.futil", "primitives/core.sv"),
    ("primitives/math.futil", "primitives/math.sv"),
    ("primitives/memories.futil", "primitives/memories.sv"),
    ("primitives/pipelined.futil", "primitives/pipelined.sv"),
    ("primitives/sync.futil", "primitives/sync.sv"),
];

/// Core primitive definitions.
pub const CORE_FUTIL: &str = include_str!("../primitives/core.futil");
/// Core primitives SystemVerilog.
pub const CORE_SV: &str = include_str!("../primitives/core.sv");

/// Binary operator primitive definitions.
pub const BINARY_OPERATORS_FUTIL: &str =
    include_str!("../primitives/binary_operators.futil");
/// Binary operator primitives SystemVerilog.
pub const BINARY_OPERATORS_SV: &str =
    include_str!("../primitives/binary_operators.sv");

/// Math primitive definitions.
pub const MATH_FUTIL: &str = include_str!("../primitives/math.futil");
/// Math primitives SystemVerilog.
pub const MATH_SV: &str = include_str!("../primitives/math.sv");

/// Sequential read and write memory primitive definitions.
pub const MEMORIES_FUTIL: &str = include_str!("../primitives/memories.futil");
/// Sequential read and write memory primitives SystemVerilog.
pub const MEMORIES_SV: &str = include_str!("../primitives/memories.sv");

/// Pipelined operator primitive definitions.
pub const PIPELINED_FUTIL: &str = include_str!("../primitives/pipelined.futil");
/// Pipelined operator primitives SystemVerilog.
pub const PIPELINED_SV: &str = include_str!("../primitives/pipelined.sv");

/// sync_reg operator primitive definitions.
pub const SYNC_FUTIL: &str = include_str!("../primitives/sync.futil");
/// sync_reg operator primitives SystemVerilog.
pub const SYNC_SV: &str = include_str!("../primitives/sync.sv");
