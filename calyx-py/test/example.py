from calyx.py_ast import *

# Variable identifiers.
lhs = CompVar("lhs")
rhs = CompVar("rhs")
sum = CompVar("sum")
add = CompVar("add")

# Create cells: three registers and an adder.
cells = [
    Cell(lhs, Stdlib.register(32), is_external=False),
    Cell(rhs, Stdlib.register(32), is_external=False),
    Cell(sum, Stdlib.register(32), is_external=False),
    Cell(add, Stdlib.op("add", 32, signed=False), is_external=False),
]

# Group names.
update_operands = "update_operands"
compute_sum = "compute_sum"

# Create the wires.
wires = [
    # Writes the values `1` and `42` to registers `lhs` and `rhs` respectively.
    Group(
        id=CompVar(update_operands),
        connections=[
            # lhs.in = 32'd1
            Connect(CompPort(lhs, "in"), ConstantPort(32, 1)),
            # rhs.in = 32'd41
            Connect(CompPort(rhs, "in"), ConstantPort(32, 41)),
            # lhs.write_en = 1'd1
            Connect(CompPort(lhs, "write_en"), ConstantPort(1, 1)),
            # rhs.write_en = 1'd1
            Connect(CompPort(rhs, "write_en"), ConstantPort(1, 1)),
            # update_operands[done] = lhs.done & rhs.done ? 1'd1;
            Connect(
                HolePort(CompVar(update_operands), "done"),
                ConstantPort(1, 1),
                guard=And(CompPort(lhs, "done"), CompPort(rhs, "done")),
            ),
        ],
    ),
    # Adds together `lhs` and `rhs` and writes it to register `sum`.
    Group(
        id=CompVar(compute_sum),
        connections=[
            # add.left = lhs.out
            Connect(CompPort(add, "left"), CompPort(lhs, "out")),
            # add.right = rhs.out
            Connect(CompPort(add, "right"), CompPort(rhs, "out")),
            # sum.write_en = 1'd1
            Connect(CompPort(sum, "write_en"), ConstantPort(1, 1)),
            # sum.in = add.out
            Connect(CompPort(sum, "in"), CompPort(add, "out")),
            # compute_sum[done] = sum.done
            Connect(HolePort(CompVar(compute_sum), "done"), CompPort(sum, "done")),
        ],
    ),
]

# Control for the component.
controls = SeqComp([Enable(update_operands), Enable(compute_sum)])

# Create the component.
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
