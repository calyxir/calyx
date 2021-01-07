import tvm
from dataclasses import dataclass
from typing import List, Dict
from types import FunctionType
from enum import Enum, IntEnum


# Note: The integer value N for Memory with dimension N is used; these should remain unchanged.
class PrimitiveType(IntEnum):
    Memory1D = 1
    Memory2D = 2
    Memory3D = 3
    Memory4D = 4
    Register = 5
    Constant = 6


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
    data_type: str


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
    wires = []  # Wire connections between components.
    cells = {}  # Instantiated sub-components. This is a mapping from {`dahlia_name`, FCell}.
    controls: FControl = None  # Control statement for this component.
    signature: FSignature = None  # Input and output ports.

    def add_cell(self, subcomponent: Cell):
        '''
        Appends a subcomponent to this component's list of FuTIL cells.
        '''
        if subcomponent == None: return
        if subcomponent.is_primitive():
            self.cells[subcomponent.primitive.name] = subcomponent
        elif subcomponent.is_relay_function():
            self.cells[subcomponent.relay_function.name] = subcomponent


@dataclass
class RelayFunctionCall:
    """
    Represents a Relay function call. This will eventually be translated to Dahlia and subsequently lowered to FuTIL.
    """
    name: str
    component_name: str
    op: str = None  # Binary operation associated with the Relay function call, if it exists.
    attributes: tvm.ir.Attrs = None  # Attributes associated with the Relay function call, e.g. `axis`, `padding`.
    lowering_function: FunctionType = None  # The function used to convert the Dahlia representation to FuTIL.
    inputs: List[Cell] = None
    output: Cell = None


@dataclass
class FCell(Cell):
    dahlia_name: str = None
    primitive: FPrimitive = None
    relay_function: RelayFunctionCall = None

    # TODO(cgyurgyik): Is there a better way to do this, such as std::variant in C++?
    def is_primitive(self): return self.primitive != None

    def is_relay_function(self): return self.relay_function != None
