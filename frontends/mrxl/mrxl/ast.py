# fmt: off
from dataclasses import dataclass
from typing import List, Union, Optional


@dataclass
class Type:
    base: str
    size: Optional[int]  # None means this is a register


# ANCHOR: decl
@dataclass
class Decl:
    input: bool  # Otherwise, output.
    name: str
    type: Type
# ANCHOR_END: decl


@dataclass
class LitExpr:
    """A constant value."""
    value: int


@dataclass
class VarExpr:
    """A variable reference."""
    name: str


BaseExpr = Union[LitExpr, VarExpr]


@dataclass
class BinExpr:
    """A binary expression. Nested expressions are not supported."""
    op: str
    lhs: BaseExpr
    rhs: BaseExpr


@dataclass
class Bind:
    dest: List[str]
    src: str


@dataclass
class Map:
    par: int
    bind: List[Bind]
    body: BinExpr


@dataclass
class Reduce:
    par: int
    bind: List[Bind]
    init: LitExpr
    body: BinExpr


# ANCHOR: stmt
@dataclass
class Stmt:
    dest: str
    op: Union[Map, Reduce]
# ANCHOR_END: stmt

    def __init__(self, dest: str, op: Union[Map, Reduce]):
        self.dest = dest
        if isinstance(op, Map):
            # Ensure that bindings for Map contain only one destination
            for bind in op.bind:
                assert len(bind.dest) == 1, "Map bindings must have one destination"
        elif isinstance(op, Reduce):
            for bind in op.bind:
                assert len(bind.dest) == 2, "Reduce bindings must have two destinations"
        self.op = op


# ANCHOR: prog
@dataclass
class Prog:
    decls: List[Decl] # Memory declarations
    stmts: List[Stmt] # Map and reduce statements
# ANCHOR_END: prog
