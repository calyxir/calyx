import calyx.py_ast as ast
import os

ast.FILEINFO_BASE_PATH=os.path.dirname(os.path.realpath(__file__))

# Variable identifiers.
lhs = ast.CompVar("lhs")
rhs = ast.CompVar("rhs")
sum = ast.CompVar("sum")
add = ast.CompVar("add")

# Create cells: three registers and an adder.
cells = [
    ast.Cell(lhs, ast.Stdlib.register(32), is_external=False),
    ast.Cell(rhs, ast.Stdlib.register(32), is_external=False),
    ast.Cell(sum, ast.Stdlib.register(32), is_external=False),
    ast.Cell(add, ast.Stdlib.op("add", 32, signed=False), is_external=False),
]

# Group names.
update_operands = "update_operands"
compute_sum = "compute_sum"

# Create the wires.
wires = [
    # Writes the values `1` and `42` to registers `lhs` and `rhs` respectively.
    ast.Group(
        id=ast.CompVar(update_operands),
        connections=[
            # lhs.in = 32'd1
            ast.Connect(ast.CompPort(lhs, "in"), ast.ConstantPort(32, 1)),
            # rhs.in = 32'd41
            ast.Connect(ast.CompPort(rhs, "in"), ast.ConstantPort(32, 41)),
            # lhs.write_en = 1'd1
            ast.Connect(ast.CompPort(lhs, "write_en"), ast.ConstantPort(1, 1)),
            # rhs.write_en = 1'd1
            ast.Connect(ast.CompPort(rhs, "write_en"), ast.ConstantPort(1, 1)),
            # update_operands[done] = lhs.done & rhs.done ? 1'd1;
            ast.Connect(
                ast.HolePort(ast.CompVar(update_operands), "done"),
                ast.ConstantPort(1, 1),
                guard=ast.And(ast.CompPort(lhs, "done"), ast.CompPort(rhs, "done")),
            ),
        ],
    ),
    # Adds together `lhs` and `rhs` and writes it to register `sum`.
    ast.Group(
        id=ast.CompVar(compute_sum),
        connections=[
            # add.left = lhs.out
            ast.Connect(ast.CompPort(add, "left"), ast.CompPort(lhs, "out")),
            # add.right = rhs.out
            ast.Connect(ast.CompPort(add, "right"), ast.CompPort(rhs, "out")),
            # sum.write_en = 1'd1
            ast.Connect(ast.CompPort(sum, "write_en"), ast.ConstantPort(1, 1)),
            # sum.in = add.out
            ast.Connect(ast.CompPort(sum, "in"), ast.CompPort(add, "out")),
            # compute_sum[done] = sum.done
            ast.Connect(ast.HolePort(ast.CompVar(compute_sum), "done"), ast.CompPort(sum, "done")),
        ],
    ),
]

# Control for the component.
controls = ast.SeqComp([ast.Enable(update_operands), ast.Enable(compute_sum)])

# Create the component.
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
