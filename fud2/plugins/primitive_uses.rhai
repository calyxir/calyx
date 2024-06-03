let primitive_uses_json = state("primitive-uses-json2", ["json"]);

op(
    "primitive-uses2",
    [get_setup("Calyx compiler")],
    get_state("calyx"),
    primitive_uses_json,
    |e, input, output| {
        e.build_cmd([output], "calyx", [input], []);
        e.arg("backend", "primitive-uses");
    },
);
