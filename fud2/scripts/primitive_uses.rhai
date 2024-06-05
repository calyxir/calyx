import "calyx" as c;

let primitive_uses_json = state("primitive-uses-json", ["json"]);

op(
    "primitive-uses",
    [c::calyx_setup],
    c::calyx_state,
    primitive_uses_json,
    |e, input, output| {
        e.build_cmd([output], "calyx", [input], []);
        e.arg("backend", "primitive-uses");
    },
);
