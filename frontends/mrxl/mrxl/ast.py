# fmt: off
from dataclasses import dataclass


@dataclass
class Type:
    """Either an array of some size, or a register."""
    base: str
    size: int | None  # If `None`, this is a register.


# ANCHOR: decl
@dataclass
class Decl:
    """Declaration of a memory."""
    input: bool  # If `False`, this is an `output`.
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


BaseExpr = LitExpr | VarExpr


@dataclass
class BinExpr:
    """A binary expression. Nested expressions are not supported."""
    operation: str
    lhs: BaseExpr
    rhs: BaseExpr


@dataclass
class Bind:
    """A binding from a source to a (list of) destination(s)."""
    dst: list[str]
    src: str


@dataclass
class Map:
    """A map operation."""
    par: int
    binds: list[Bind]
    body: BinExpr


@dataclass
class Reduce:
    """A reduce operation."""
    par: int
    binds: list[Bind]
    init: LitExpr
    body: BinExpr


# ANCHOR: stmt
@dataclass
class Stmt:
    """A statement in the program."""
    dst: str
    operation: Map | Reduce
# ANCHOR_END: stmt

    def __init__(self, dst: str, operation: Map | Reduce):
        self.dst = dst
        if isinstance(operation, Map):
            # Ensure that bindings for Map contain only one destination
            for bind in operation.binds:
                assert len(bind.dst) == 1, "Map bindings must have one destination"
        elif isinstance(operation, Reduce):
            for bind in operation.binds:
                assert len(bind.dst) == 2, "Reduce bindings must have two destinations"
        self.operation = operation


# ANCHOR: prog
@dataclass
class Prog:
    """A MrXL program."""
    decls: list[Decl]  # Memory declarations
    stmts: list[Stmt]  # Map and reduce statements
# ANCHOR_END: prog
