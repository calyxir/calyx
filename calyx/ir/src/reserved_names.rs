/// Names that are reserved in Calyx and Verilog.
pub const RESERVED_NAMES: &[&str] = &[
    // Calyx keywords
    "invoke",
    "par",
    "seq",
    "if",
    "while",
    "with",
    "component",
    "primitive",
    "extern",
    // Verilog keywords
    "reg",
    "wire",
    "always",
    "posedge",
    "negedge",
    "logic",
    "tri",
    "input",
    "output",
    "if",
    "generate",
    "var",
    "go",
    "done",
    "clk",
    "and",
    "process",
    "assign",
    "automatic",
    "begin",
    "buf",
    "bufif0",
    "bufif1",
    "case",
    "casex",
    "casez",
    "cell",
    "cmos",
    "config",
    "deassign",
    "default",
    "defparam",
    "design",
    "disable",
    "edge",
    "else",
    "end",
    "endcase",
    "endconfig",
    "endfunction",
    "endgenerate",
    "endmodule",
    "endprimitive",
    "endspecify",
    "endtable",
    "endtask",
    "event",
    "for",
    "forever",
    "fork",
    "function",
    "genvar",
    "highz0",
    "highz1",
    "ifnone",
    "incdir",
    "include",
    "initial",
    "inout",
    "instance",
    "integer",
    "join",
    "large",
    "liblist",
    "library",
    "localparam",
    "macromodule",
    "medium",
    "module",
    "nmos",
    "nor",
    "noshowcancelledno",
    "not",
    "notif0",
    "notif1",
    "or",
    "parameter",
    "pmos",
    "primitive",
    "pull0",
    "pull1",
    "pulldown",
    "pullup",
    "pulsestyle_oneventglitch",
    "pulsestyle_ondetectglitch",
    "remos",
    "real",
    "realtime",
    "release",
    "repeat",
    "rnmos",
    "rpmos",
    "rtran",
    "rtranif0",
    "rtranif1",
    "scalared",
    "showcancelled",
    "signed",
    "small",
    "specify",
    "specparam",
    "strong0",
    "strong1",
    "supply0",
    "supply1",
    "table",
    "task",
    "time",
    "tran",
    "tranif0",
    "tranif1",
    "tri0",
    "tri1",
    "triand",
    "trior",
    "trireg",
    "unsigned",
    "use",
    "vectored",
    "wand",
    "weak0",
    "weak1",
    "while",
    "wor",
    "xnor",
    "xor",
    "wait",
    "break",
];
