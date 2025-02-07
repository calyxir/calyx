import calyx.py_ast as ast
import os

ast.FILEINFO_BASE_PATH=os.path.dirname(os.path.realpath(__file__))

x = ast.CompVar("x")
lt = ast.CompVar("lt")

cells = [
    ast.Cell(lt, ast.Stdlib.op("lt", 32, signed=False), is_external=False),
    ast.Cell(x, ast.Stdlib.register(32), is_external=False),
]

true = "true"
false = "false"
cond = ast.CompVar("cond")

wires = [
    ast.Group(
        id=cond,
        connections=[
            ast.Connect(ast.CompPort(lt, "left"), ast.ConstantPort(32, 0)),
            ast.Connect(ast.CompPort(lt, "right"), ast.ConstantPort(32, 1)),
            ast.Connect(ast.HolePort(cond, "done"), ast.ConstantPort(1, 1)),
        ],
    ),
    ast.Group(
        id=ast.CompVar(true),
        connections=[
            ast.Connect(ast.CompPort(x, "in"), ast.ConstantPort(32, 1)),
            ast.Connect(ast.CompPort(x, "write_en"), ast.ConstantPort(1, 1)),
            ast.Connect(ast.HolePort(ast.CompVar(true), "done"), ast.CompPort(x, "done")),
        ],
    ),
    ast.Group(
        id=ast.CompVar(false),
        connections=[
            ast.Connect(ast.CompPort(x, "in"), ast.ConstantPort(32, 0)),
            ast.Connect(ast.CompPort(x, "write_en"), ast.ConstantPort(1, 1)),
            ast.Connect(ast.HolePort(ast.CompVar(false), "done"), ast.CompPort(x, "done")),
        ],
    ),
]

controls = ast.If(ast.CompPort(lt, "out"), cond, ast.Enable(true), ast.Enable(false))

main_component = ast.Component(
    name="main",
    attributes=set(),
    inputs=[],
    outputs=[],
    structs=cells + wires,
    controls=controls,
)

# Create the Calyx program.
program = ast.Program(
    imports=[
        ast.Import("primitives/core.futil"),
        ast.Import("primitives/binary_operators.futil"),
    ],
    components=[main_component],
)

# Emit the code.
program.emit()
