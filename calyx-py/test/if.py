from futil.ast import *

stdlib = Stdlib()
x = CompVar('x')
lt = CompVar('lt')

cells = [
    LibDecl(lt, stdlib.op('lt', 32, signed=False), is_external=False),
    LibDecl(x, stdlib.register(32), is_external=False)
]

true = 'true'
false = 'false'
cond = CompVar('cond')

wires = [
    Group(
        id=cond,
        connections=[
            Connect(ConstantPort(32, 1), CompPort(lt, 'left')),
            Connect(ConstantPort(32, 1), CompPort(lt, 'right')),
            Connect(ConstantPort(1, 1), HolePort(cond, 'done'))
        ]
    ),
    Group(
        id=CompVar(true),
        connections=[
            Connect(ConstantPort(32, 1), CompPort(x, 'in')),
            Connect(ConstantPort(1, 1), CompPort(x, 'write_en')),
            Connect(CompPort(x, 'done'), HolePort(CompVar(true), 'done'))
        ]
    ),
    Group(
        id=CompVar(false),
        connections=[
            Connect(ConstantPort(32, 0), CompPort(x, 'in')),
            Connect(ConstantPort(1, 1), CompPort(x, 'write_en')),
            Connect(CompPort(x, 'done'), HolePort(CompVar(false), 'done'))
        ]
    )
]

controls = ControlEntry(
    ControlEntryType.Seq,
    stmts=[
        If(
            CompPort(lt, 'out'),
            cond,
            Enable(true),
            Enable(false)
        )
    ]
)

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
