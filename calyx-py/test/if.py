from calyx.py_ast import *

x = CompVar("x")
lt = CompVar("lt")

cells = [
    Cell(lt, Stdlib.op("lt", 32, signed=False), is_external=False),
    Cell(x, Stdlib.register(32), is_external=False),
]

true = "true"
false = "false"
cond = CompVar("cond")

wires = [
    Group(
        id=cond,
        connections=[
            Connect(CompPort(lt, "left"), ConstantPort(32, 0)),
            Connect(CompPort(lt, "right"), ConstantPort(32, 1)),
            Connect(HolePort(cond, "done"), ConstantPort(1, 1)),
        ],
    ),
    Group(
        id=CompVar(true),
        connections=[
            Connect(CompPort(x, "in"), ConstantPort(32, 1)),
            Connect(CompPort(x, "write_en"), ConstantPort(1, 1)),
            Connect(HolePort(CompVar(true), "done"), CompPort(x, "done")),
        ],
    ),
    Group(
        id=CompVar(false),
        connections=[
            Connect(CompPort(x, "in"), ConstantPort(32, 0)),
            Connect(CompPort(x, "write_en"), ConstantPort(1, 1)),
            Connect(HolePort(CompVar(false), "done"), CompPort(x, "done")),
        ],
    ),
]

controls = If(CompPort(lt, "out"), cond, Enable(true), Enable(false))

main_component = Component(
    name="main",
    attributes=[],
    inputs=[],
    outputs=[],
    structs=cells + wires,
    controls=controls,
)

# Create the Calyx program.
program = Program(
    imports=[
        Import("primitives/core.futil"),
        Import("primitives/binary_operators.futil"),
    ],
    components=[main_component],
)

# Emit the code.
program.emit()
