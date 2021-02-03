from futil.ast import *

stdlib = Stdlib()
temp = CompVar("temp")

foo_cells = [Cell(temp, stdlib.register(32))]

foo_wires = [
    Group(
        CompVar("let"),
        [
            Connect(ThisPort(CompVar("a")), CompPort(temp, "in")),
            Connect(ConstantPort(1, 1), CompPort(temp, "write_en")),
            Connect(CompPort(temp, "done"), HolePort(CompVar("let"), "done")),
        ],
        1,
    ),
    Connect(CompPort(temp, "out"), ThisPort(CompVar("out"))),
]

foo_component = Component(
    name="foo",
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
    Cell(b, stdlib.register(32)),
    Cell(c, stdlib.register(32)),
    Cell(const, stdlib.constant(32, 1)),
    Cell(foo, CompInst("foo", [])),
]

wires = [
    Group(
        CompVar("write_constant"),
        [
            Connect(CompPort(const, "out"), CompPort(b, "in")),
            Connect(ConstantPort(1, 1), CompPort(b, "write_en")),
            Connect(CompPort(b, "done"), HolePort(CompVar("write_constant"), "done")),
        ],
        1,
    ),
    Group(
        CompVar("save_foo"),
        [
            Connect(CompPort(foo, "out"), CompPort(c, "in")),
            Connect(ConstantPort(1, 1), CompPort(c, "write_en")),
            Connect(CompPort(c, "done"), HolePort(CompVar("save_foo"), "done")),
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
    inputs=[],
    outputs=[],
    structs=cells + wires,
    controls=SeqComp(controls),
)

# Create the FuTIL program.
program = Program(
    imports=[Import("primitives/std.lib")], components=[foo_component, main_component]
)

# Emit the code.
program.emit()
