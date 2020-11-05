from dataclasses import dataclass
from typing import List, Dict
from types import FunctionType
from enum import Enum


class PrimitiveType(Enum):
    Register = 1
    Constant = 2
    Memory1D = 3
    Memory2D = 4
    Memory3D = 5


class ControlType(Enum):
    Seq = 1
    Par = 2


@dataclass
class FPrimitive:
    '''
    Represents a FuTIL primitive.
    `data` represents the necessary data to instantiate said component.
    '''
    name: str
    data: List[int]
    type: PrimitiveType


@dataclass
class FPortDef:
    '''
    The definition of an input/output FuTIL port.
    '''
    name: str
    bitwidth: int


@dataclass
class FSignature:
    '''
    Represents the signature of a component. This contains a list
    of input ports and a list of output ports.
    '''
    inputs: List[FPortDef]
    outputs: List[FPortDef]


@dataclass
class FWire:
    src: str  # FGuard
    dst: str  # FPort


@dataclass
class FGroup:
    '''
    Represents a FuTIL group.
    '''
    name: str
    wires: List[FWire]
    attributes: Dict[str, int]


@dataclass
class FConnection:
    group: FGroup = None
    wire: FWire = None

    def is_wire(self):
        return self.wire != None

    def is_group(self):
        return self.group != None


@dataclass
class ControlType:
    stmts: List[str]


@dataclass
class Seq(ControlType):
    name: str = "seq"


@dataclass
class FControl:
    '''
    Represents the AST nodes for the FuTIL control.
    TODO(cgyurgyik): Break this into different components, i.e. Seq, While, If, ...
    '''
    stmts: List[ControlType]


# TODO(cgyurgyik): A not-so-pretty way to overcome interdependencies between
# FuTIL cells and FuTIL components.
@dataclass
class Cell:
    pass


@dataclass
class FComponent:
    '''
    Represents a FuTIL component.
    '''
    name: str
    cells: List[Cell]  # Instantiated sub-components.
    wires: List[FConnection]  # Wire connections between components.
    controls: FControl = None  # Control statement for this component.
    signature: FSignature = None  # Input and output ports.

    def contains_primitive(self, name: str):
        '''
        Determines whether this component contains a primitive with the given name.
        '''
        # TODO(cgyurgyik): Rethink data structure here.
        for cell in self.cells:
            if not cell.is_primitive(): continue
            if cell.primitive.name == name: return True
        return False

    def add_cell(self, subcomponent: Cell):
        '''
        Appends a subcomponent to this component's list of FuTIL cells.
        '''
        if not subcomponent.is_primitive():
            self.cells.append(subcomponent)
            return
        if self.contains_primitive(subcomponent.primitive.name): return
        self.cells.append(subcomponent)


@dataclass
class DahliaDeclaration:
    decl_name: str
    component_name: str
    op: str = None
    inputs: List[Cell] = None
    output: Cell = None
    function: FunctionType = None
    program: str = None

    def invoke(self):
        self.program = self.function(self)


@dataclass
class FDeclaration:
    '''
    Represents a FuTIL declaration.
    '''
    name: str
    intermediary_inputs: List[Cell] = None
    intermediary_output: Cell = None
    component: FComponent = None


@dataclass
class FCell(Cell):
    dahlia_name: str = None
    primitive: FPrimitive = None
    declaration: FDeclaration = None
    dahlia_declaration: DahliaDeclaration = None

    def is_primitive(self): return self.primitive != None

    def is_declaration(self): return self.declaration != None

    def is_dahlia_declaration(self): return self.dahlia_declaration != None
