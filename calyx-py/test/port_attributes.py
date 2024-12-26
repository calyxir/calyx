import calyx.builder as cb


def insert_foo_component(prog):
    comp = prog.component("foo")

    foo_inputs = [
        ("in_1", 1),
        ("in_2", 2, ["data"]),
        ("in_3", 2, ["data", ("write_together", 1)]),
    ]

    cb.add_comp_ports(comp, foo_inputs, [])

    comp.output("out_1", 1)
    # ANCHOR: port_attributes
    comp.output("out_2", 1, ["data"])
    comp.output("out_3", 1, ["data", ("done", 1)])
    # ANCHOR_END: port_attributes


if __name__ == "__main__":
    prog = cb.Builder()
    insert_foo_component(prog)
    prog.program.emit()
