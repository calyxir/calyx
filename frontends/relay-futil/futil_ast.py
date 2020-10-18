from dataclasses import dataclass
from typing import List, Dict
from enum import Enum
import textwrap

class PrimitiveType(Enum):
    NoneType = 0
    Register = 1
    Constant = 2

@dataclass
class FPrimitive:
    '''
    Represents a FuTIL primitive.
    `data` represents the necessary data to instantiate said component.
    '''
    name: str
    data: List[int]
    type: PrimitiveType = PrimitiveType.NoneType


@dataclass
class FPortDef:
    '''
    The definition of an input/output FuTIL port.
    '''
    name: str
    width: int


@dataclass
class FPort:
    '''
    Statement which refers to a port on a subcomponent.
    This is distinct from a `FPortDef`, which defines the port.
    '''
    ComponentName: str
    This: str
    Hole: str

    def port_name(self, s):
        '''
        :param s: Returns the name of the port being referenced. This must be "This" or "Hole".
        :return: FPort.This or FPort.Hole.
        '''
        assert (s == "This" or s == "Hole")
        if s == "This":
            return This
        elif s == "Hole":
            return Hole


@dataclass
class FSignature:
    '''
    Represents the signature of a component. This contains a list
    of input ports and a list of output ports.
    '''
    inputs: List[FPortDef]
    outputs: List[FPortDef]


@dataclass
class FCell:
    primitive: FPrimitive


@dataclass
class Atom:
    '''
    Atomic operations used in guard conditions and RHS of the guarded assignments.
    '''
    port: FPort
    num: int  # TODO(cgyurgyik): This uses a Bitnum structure.


@dataclass
class FGuard:
    guard_expression: str  # "And", "Or", "Eq", "Neq", ...
    atom: Atom


@dataclass
class FWire:
    src: FGuard
    dest: FPort


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
    group: FGroup
    wire: FWire


@dataclass
class FControl:
    '''
    Represents the AST nodes for the FuTIL control.
    TODO(cgyurgyik): Break this into different components, i.e. Seq, While, If, ...
    '''
    # port: FPort
    cond: str
    comp: str
    stmts: List[str]


@dataclass
class FComponent:
    '''
    Represents a FuTIL component.
    '''
    cells: List[FCell]  # Instantiated sub-components.
    wires: List[FConnection]  # Wire connections between components.
    name: str = "main"
    signature: FSignature = FSignature(inputs=[], outputs=[])  # Input and output ports.
    control: FControl = FControl(cond="", comp="", stmts=[])  # Control statement for this component.

    def add_wire(self, subcomponent):
        '''
        Appends a subcomponent to this component's list of FuTIL cells.
        '''
        self.cells.append(subcomponent)

def build_assigment(dst: FPort, src: FPort, guard: FGuard):
    assert False, "Unimplemented"