from futil.futil_ast import *

stdlib = Stdlib()

# Variable identifiers.
lhs = CompVar('lhs')
rhs = CompVar('rhs')
sum = CompVar('sum')
add = CompVar('add')

# Create cells: three registers and an adder.
cells = [
    LibDecl(lhs, stdlib.register(32), is_external=False),
    LibDecl(rhs, stdlib.register(32), is_external=False),
    LibDecl(sum, stdlib.register(32), is_external=False),
    LibDecl(add, stdlib.op('add', 32), is_external=False)
]

# Group names.
do_write = 'do_write'
save_sum = 'save_sum'

# Create the wires.
wires = [
    # Writes the values `1` and `42` to registers `lhs` and `rhs` respectively.
    Group(
        id=CompVar(do_write),
        connections=[
            # lhs.in = 32'd1
            Connect(ConstantPort(32, 1), CompPort(lhs, 'in')),
            # rhs.in = 32'd41
            Connect(ConstantPort(32, 41), CompPort(rhs, 'in')),
            # lhs.write_en = 1'd1
            Connect(ConstantPort(1, 1), CompPort(lhs, 'write_en')),
            # rhs.write_en = 1'd1
            Connect(ConstantPort(1, 1), CompPort(rhs, 'write_en')),
            # write[done] = lhs.done & rhs.done ? 1'd1;
            Connect(
                ConstantPort(1, 1),
                HolePort(CompVar(do_write), 'done'),
                And(CompPort(lhs, 'done'), CompPort(rhs, 'done'))
            )
        ]
    ),
    # Adds together `lhs` and `rhs` and saves it to register `sum`.
    Group(
        id=CompVar(save_sum),
        connections=[
            # add.left = lhs.out
            Connect(CompPort(lhs, 'out'), CompPort(add, 'left')),
            # add.right = rhs.out
            Connect(CompPort(rhs, 'out'), CompPort(add, 'right')),
            # sum.write_en = 1'd1
            Connect(ConstantPort(1, 1), CompPort(sum, 'write_en')),
            # sum.in = add.out
            Connect(CompPort(add, 'out'), CompPort(sum, 'in')),
            # save_sum[done] = sum.done
            Connect(
                CompPort(sum, 'done'),
                HolePort(CompVar(save_sum), 'done')
            )
        ]
    )
]

# Control for the component.
controls = ControlEntry(
    ControlEntryType.Seq,
    [Enable(do_write), Enable(save_sum)]
)

# Create the component.
main_component = Component(
    name='main',
    inputs=[],
    outputs=[],
    structs=cells + wires,
    controls=controls
)

# Create the FuTIL program.
program = Program(
    imports=[Import('primitives/std.lib')],
    components=[main_component]
)

# Emit the code.
program.emit()
