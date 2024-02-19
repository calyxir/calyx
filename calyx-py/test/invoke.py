from calyx.py_ast import (
    CompVar,
    Cell,
    Stdlib,
    CompPort,
    ThisPort,
    Connect,
    HolePort,
    Component,
    PortDef,
    SeqComp,
    Group,
    ConstantPort,
    Enable,
    CompInst,
    Invoke,
    Import,
    Program,
)

temp = CompVar("temp")

foo_cells = [Cell(temp, Stdlib.register(32))]

foo_wires = [
    Group(
        CompVar("let"),
        [
            Connect(CompPort(temp, "in"), ThisPort(CompVar("a"))),
            Connect(CompPort(temp, "write_en"), ConstantPort(1, 1)),
            Connect(HolePort(CompVar("let"), "done"), CompPort(temp, "done")),
        ],
        1,
    ),
    Connect(ThisPort(CompVar("out")), CompPort(temp, "out")),
]

foo_component = Component(
    name="foo",
    attributes=[],
    inputs=[PortDef(CompVar("a"), 32)],
    outputs=[PortDef(CompVar("out"), 32)],
    structs=foo_cells + foo_wires,
    controls=SeqComp([Enable("let")]),
)

b = CompVar("b")
c = CompVar("c")
const = CompVar("cst")
foo = CompVar("foo0")

cells = [
    Cell(b, Stdlib.register(32)),
    Cell(c, Stdlib.register(32)),
    Cell(const, Stdlib.constant(32, 1)),
    Cell(foo, CompInst("foo", [])),
]

wires = [
    Group(
        CompVar("write_constant"),
        [
            Connect(CompPort(b, "in"), CompPort(const, "out")),
            Connect(CompPort(b, "write_en"), ConstantPort(1, 1)),
            Connect(HolePort(CompVar("write_constant"), "done"), CompPort(b, "done")),
        ],
        1,
    ),
    Group(
        CompVar("save_foo"),
        [
            Connect(CompPort(c, "in"), CompPort(foo, "out")),
            Connect(CompPort(c, "write_en"), ConstantPort(1, 1)),
            Connect(HolePort(CompVar("save_foo"), "done"), CompPort(c, "done")),
        ],
    ),
]

controls = [
    Enable("write_constant"),
    Invoke(id=foo, in_connects=[("a", CompPort(b, "out"))], out_connects=[]),
    Enable("save_foo"),
]

main_component = Component(
    name="main",
    attributes=[],
    inputs=[],
    outputs=[],
    structs=cells + wires,
    controls=SeqComp(controls),
)

# Create the Calyx program.
program = Program(
    imports=[
        Import("primitives/core.futil"),
        Import("primitives/binary_operators.futil"),
    ],
    components=[foo_component, main_component],
)

# Emit the code.
program.emit()
