# fmt: off
from dataclasses import dataclass
from typing import List, Union, Optional
from enum import Enum


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


class BinOp(Enum):
    ADD = "add"
    MUL = "mul"


@dataclass
class BinExpr:
    """A binary expression."""
    op: BinOp
    lhs: "Expr"
    rhs: "Expr"


@dataclass
class LitExpr:
    """A constant value."""
    value: int


@dataclass
class VarExpr:
    """A variable reference."""
    name: str


Expr = Union[BinExpr, LitExpr, VarExpr]


@dataclass
class Bind:
    dest: List[str]
    src: str


@dataclass
class Map:
    par: int
    bind: List[Bind]
    body: Expr


@dataclass
class Reduce:
    par: int
    bind: List[Bind]
    init: Expr
    body: Expr


# ANCHOR: stmt
@dataclass
class Stmt:
    dest: str
    op: Union[Map, Reduce]


# ANCHOR_END: stmt


# ANCHOR: prog
@dataclass
class Prog:
    decls: List[Decl]
    stmts: List[Stmt]


# ANCHOR_END: prog
