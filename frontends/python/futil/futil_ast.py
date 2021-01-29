from __future__ import annotations  # Used for circular dependencies.
from dataclasses import dataclass
from enum import Enum
from typing import Union
from futil.futil_utils import block


@dataclass
class Emittable:
    def doc(self) -> str:
        assert 0, f'`doc` not implemented for {type(self).__name__}'

    def emit(self):
        print(self.doc())


### Program ###

@dataclass
class Import(Emittable):
    filename: str

    def __init__(self, filename: str):
        self.filename = filename

    def doc(self) -> str:
        return f'import "{self.filename}";'


@dataclass
class Program(Emittable):
    imports: List[Import]
    components: List[Component]

    def __init__(self, imports: List[Import], components: List[Component]):
        self.imports = imports
        self.components = components

    def doc(self) -> str:
        imports = '\n'.join([i.doc() for i in self.imports])
        components = '\n'.join([c.doc() for c in self.components])
        return f'{imports}\n{components}'


### Component ###

@dataclass
class Component:
    name: str
    inputs: list[PortDef]
    outputs: list[PortDef]
    wires: list[Structure]
    cells: list[Structure]
    controls: Control

    def __init__(self, name: str,
                 inputs: list[PortDef], outputs: list[PortDef],
                 structs: list[Structure], controls: Control):
        self.inputs = inputs
        self.outputs = outputs
        self.name = name
        self.controls = controls

        # Partition cells and wires.
        is_cell = lambda x: isinstance(x, LibDecl) or isinstance(x, CompDecl)
        self.cells = [s for s in structs if is_cell(s)]
        self.wires = [s for s in structs if not is_cell(s)]

    def doc(self) -> str:
        ins = ', '.join([s.doc() for s in self.inputs])
        outs = ', '.join([s.doc() for s in self.outputs])
        signature = f'component {self.name}({ins}) -> ({outs})'
        cells = block('cells', [c.doc() for c in self.cells])
        wires = block('wires', [w.doc() for w in self.wires])
        controls = block('control', [self.controls.doc()])
        return block(signature, [cells, wires, controls])


### Ports ###

@dataclass
class Port(Emittable):
    pass


@dataclass
class CompPort(Port):
    id: CompVar
    name: str

    def __init__(self, id: CompVar, name: str):
        assert isinstance(id, CompVar), f'id: {id} is not a CompVar.'
        self.id = id
        self.name = name

    def doc(self) -> str:
        return f'{self.id.doc()}.{self.name}'


@dataclass
class ThisPort(Port):
    id: CompVar

    def __init__(self, id: CompVar):
        self.id = id

    def doc(self) -> str:
        return self.id.doc()


@dataclass
class HolePort(Port):
    id: CompVar
    name: str

    def __init__(self, id: CompVar, name: str):
        self.id = id
        self.name = name

    def doc(self) -> str:
        return f'{self.id.doc()}[{self.name}]'


@dataclass
class ConstantPort(Port):
    width: int
    value: int

    def __init__(self, width: int, value: int):
        self.width = width
        self.value = value

    def doc(self) -> str:
        return f'{self.width}\'d{self.value}'


@dataclass
class CompVar(Emittable):
    name: str

    def __init__(self, name: str):
        self.name = name

    def doc(self) -> str:
        return self.name

    def port(self, port: str) -> CompPort:
        return CompPort(self, port)

    def add_suffix(self, suffix: str) -> CompVar:
        return CompVar(f'{self.name}{suffix}')


@dataclass
class PortDef(Emittable):
    id: CompVar
    width: int

    def __init__(self, id: CompVar, width: int):
        self.id = id
        self.width = width

    def doc(self) -> str:
        return f'{self.id.doc()}: {self.width}'


### Structure ###
@dataclass
class Structure(Emittable):
    pass


@dataclass
class CompDecl(Structure):
    id: CompVar
    comp: CompVar

    def __init__(self, id: CompVar, comp: CompVar):
        self.id = id
        self.comp = comp

    def doc(self) -> str:
        return f'{self.id.doc()} = {self.comp.doc()};'


@dataclass
class LibDecl(Structure):
    id: CompVar
    comp: CompInst
    is_external: bool

    def __init__(self, id: CompVar, comp: CompInst, is_external: bool = False):
        self.id = id
        self.comp = comp
        self.is_external = is_external

    def doc(self) -> str:
        external = '@external(1) ' if self.is_external else ''
        return f'{external}{self.id.doc()} = prim {self.comp.doc()};'


@dataclass
class Connect(Structure):
    src: Port
    dest: Port
    guard: GuardExpr

    def __init__(self, src: Port, dest: Port, guard: GuardExpr = None):
        self.src = src
        self.dest = dest
        self.guard = guard

    def doc(self) -> str:
        source = self.src.doc() if self.guard == None else f'{self.guard.doc()} ? {self.src.doc()}'
        return f'{self.dest.doc()} = {source};'


@dataclass
class Group(Structure):
    id: CompVar
    connections: list[Connect]
    static_delay: int

    def __init__(self, id: CompVar, connections: list[Connect], static_delay: int = None):
        self.id = id
        self.connections = connections
        self.static_delay = static_delay

    def doc(self) -> str:
        static_delay_attr = '' if self.static_delay == None else f'<"static"={self.static_delay}>'
        return block(f'group {self.id.doc()}{static_delay_attr}',
                     [c.doc() for c in self.connections])


@dataclass
class CompInst(Emittable):
    id: str
    args: list[int]

    def __init__(self, id: str, args: list[int]):
        self.id = id
        self.args = args

    def doc(self) -> str:
        args = ', '.join([str(x) for x in self.args])
        return f'{self.id}({args})'


### Guard Expressions ###
@dataclass
class GuardExpr(Emittable):
    pass


@dataclass
class Atom(GuardExpr):
    item: Port

    def __init__(self, item: Port):
        self.item = item

    def doc(self) -> str:
        return self.item.doc()


@dataclass
class Not(GuardExpr):
    inner: GuardExpr

    def __init__(self, inner: GuardExpr):
        self.inner = inner

    def doc(self) -> str:
        return f'!{self.inner.doc()}'


@dataclass
class And(GuardExpr):
    left: GuardExpr
    right: GuardExpr

    def __init__(self, left: GuardExpr, right: GuardExpr):
        self.left = left
        self.right = right

    def doc(self) -> str:
        return f'{self.left.doc()} & {self.right.doc()}'


@dataclass
class Or(GuardExpr):
    left: GuardExpr
    right: GuardExpr

    def __init__(self, left: GuardExpr, right: GuardExpr):
        self.left = left
        self.right = right

    def doc(self) -> str:
        return f'{self.left.doc()} | {self.right.doc()}'


### Control ###

class ControlEntryType(Enum):
    Seq = 'seq'
    Par = 'par'


# TODO(cgyurgyik): AST support for `While`, `If`, and `Empty`.
@dataclass
class Control(Emittable):
    pass

@dataclass
class ControlEntry(Control):
    stmts: list[ControlOrEnable]
    entry: ControlEntryType

    def __init__(self, entry: ControlEntryType, stmts: list[ControlOrEnable]):
        self.entry = entry
        self.stmts = stmts

    def doc(self):
        return block(self.entry.value, [s.doc() for s in self.stmts])


@dataclass
class Enable(Emittable):
    stmt: str

    def __init__(self, stmt):
        self.stmt = stmt

    def doc(self) -> str:
        return f'{self.stmt};'


@dataclass
class ControlOrEnable(Emittable):
    ControlOrEnableType = Union[Control, Enable]
    stmt: ControlOrEnableType

    def __init__(self, s: ControlOrEnableType):
        assert isinstance(s, Control) or isinstance(s, Enable), f'{s} should be a Control or Enable.'
        self.stmt = stmt

    def doc(self) -> str:
        return self.doc()


@dataclass
class SeqComp(Control):
    stmts: list[ControlOrEnable]

    def __init__(self, stmts: list[ControlOrEnable]):
        self.stmts = stmts

    def doc(self) -> str:
        return block('seq', [s.doc() for s in self.stmts])


@dataclass
class ParComp(Control):
    stmts: list[ControlOrEnable]

    def __init__(self, stmts: list[ControlOrEnable]):
        self.stmts = stmts

    def doc(self) -> str:
        return block('par', [s.doc() for s in self.stmts])


@dataclass
class Invoke(Control):
    id: CompVar
    args: List[Port]
    params: List[CompVar]

    def __init__(self, id: CompVar, args: List[Port],
                 params: List[CompVar]):
        self.id = id
        self.args = args
        self.params = params

    def doc(self) -> str:
        definitions = [
            f'{x[0].doc()}={x[1].doc()}' for x in zip(self.params, self.args)
        ]
        return f'invoke {self.id.doc()}({", ".join(definitions)})();'


### Standard Library ###

# TODO(cgyurgyik): AST support for fixed point operations (signed and unsigned).
@dataclass
class Stdlib:
    def register(self, bitwidth: int):
        return CompInst('std_reg', [bitwidth])

    def constant(self, bitwidth: int, value: int):
        return CompInst('std_const', [bitwidth, value])

    def op(self, op: str, bitwidth: int):
        return CompInst(f'std_{op}', [bitwidth])

    def identity(self, op: str, bitwidth: int):
        return CompInst('std_id', [bitwidth])

    def slice(self, op: str, in_: int, out: int):
        return CompInst('std_slice', [in_, out])

    def mem_d1(self, bitwidth: int, size: int, idx_size: int):
        return CompInst('std_mem_d1', [bitwidth, size, idx_size])

    def mem_d2(self, bitwidth: int,
               size0: int, size1: int,
               idx_size0: int, idx_size1: int):
        return CompInst('std_mem_d2', [bitwidth, size0, size1,
                                       idx_size0, idx_size1])

    def mem_d3(self, bitwidth: int,
               size0: int, size1: int, size2: int,
               idx_size0: int, idx_size1: int, idx_size2: int):
        return CompInst('std_mem_d3', [bitwidth, size0, size1, size2,
                                       idx_size0, idx_size1, idx_size2])

    def mem_d4(self, bitwidth: int,
               size0: int, size1: int, size2: int, size3: int,
               idx_size0: int, idx_size1: int, idx_size2: int, idx_size3: int):
        return CompInst('std_mem_d4', [bitwidth, size0, size1, size2, size3,
                                       idx_size0, idx_size1, idx_size2, idx_size3])
