from __future__ import annotations
from typing import overload, Generic, TypeVar
from egglog import *


class Boolean(Expr):
    # Taken from (don't totally understand what is going on):
    # https://github.com/egraphs-good/egglog-python/blob/main/python/egglog/exp/array_api.py
    @method(preserve=True)
    def __bool__(self) -> bool:
        return try_evaling(self, self.bool)

    @property
    def bool(self) -> Bool: ...

    def __or__(self, other: BooleanLike) -> Boolean: ...

    def __and__(self, other: BooleanLike) -> Boolean: ...

    def __xor__(self, other: BooleanLike) -> Boolean: ...

    def __not__(self) -> Boolean: ...


BooleanLike = Boolean | bool

TRUE = constant("TRUE", Boolean)
FALSE = constant("FALSE", Boolean)
converter(bool, Boolean, lambda x: TRUE if x else FALSE)


class Num(Expr):
    def __init__(self, value: i64Like) -> None: ...

    def __add__(self, other: Num) -> Num: ...

    def __sub__(self, other: Num) -> Num: ...

    def __le__(self, other: Num) -> Boolean: ...

    def __lt__(self, other: Num) -> Boolean: ...

    def __gt__(self, other: Num) -> Boolean: ...

    def __ge__(self, other: Num) -> Boolean: ...

    def __eq__(self, other: Num) -> Boolean: ...

    # Taken from:
    # https://github.com/egraphs-good/egglog-python/blob/main/python/egglog/exp/array_api.py
    @method(preserve=True)
    def __ne__(self, other: Int) -> bool:  # type: ignore[override]
        return not (self == other)

    @method(preserve=True)
    def __bool__(self) -> bool:
        return bool(int(self))


converter(i64, Num, Num)


class OptionalNum(Expr):
    def __init__(self, x: Num) -> None: ...

    @classmethod
    def none(cls) -> OptionalNum: ...


converter(Num, OptionalNum, OptionalNum)
converter(None, OptionalNum, lambda _: OptionalNum.none())


class GuardExpr(Expr):
    def __init__(self) -> None: ...

    @classmethod
    def none(cls) -> GuardExpr: ...


@function
def Atom(item: Port) -> GuardExpr: ...


@function
def Not(inner: GuardExpr) -> GuardExpr: ...


@function
def And(lhs: GuardExpr, rhs: GuardExpr) -> GuardExpr: ...


@function
def Or(lhs: GuardExpr, rhs: GuardExpr) -> GuardExpr: ...


@function
def Eq(lhs: GuardExpr, rhs: GuardExpr) -> GuardExpr: ...


converter(None, GuardExpr, lambda _: GuardExpr.none())


############################################################
########################## Calyx ###########################
############################################################


class Component(Expr):
    def __init__(self,
                 name: StringLike,
                 inputs: Vec[PortDef],
                 outputs: Vec[PortDef],
                 cells: Vec[Cell],
                 wires: Vec[Group],  # TODO(cgyurgyik): `Connects` are allowed.
                 controls: Control,
                 ) -> None: ...


class CompInst(Expr):
    def __init__(self, id: StringLike, args: Vec[Num]) -> None: ...

    def __eq__(self, other: CompInst) -> Boolean: ...


class Cell(Expr):
    def __init__(self, id: CompVar, comp: CompInst) -> None: ...


class CompVar(Expr):
    def __init__(self, name: StringLike) -> None: ...


class PortDef(Expr):
    def __init__(self, id: CompVar, width: Num) -> None: ...


class Port(Expr):
    def __init__(self) -> None: ...


@function
def CompPort(id: CompVar, name: StringLike) -> Port: ...


@function
def HolePort(id: CompVar, name: StringLike) -> Port: ...


@function
def ConstantPort(width: Num, value: Num) -> Port: ...


class Connect(Expr):
    def __init__(self, dest: Port, src: Port,
                 guard: GuardExpr = GuardExpr.none()) -> None: ...


class Group(Expr):
    def __init__(self, name: CompVar, connects: Vec[Connect],
                 promotable: OptionalNum = OptionalNum.none()) -> None: ...


class Control(Expr):
    def __init__(self) -> None: ...

    @classmethod
    def none(cls) -> Control: ...


@function
def Enable(name: StringLike, promotable: OptionalNum = OptionalNum.none()) -> Control: ...


@function
def Seq(list: ControlList) -> Control: ...


@function
def Par(list: ControlList) -> Control: ...


converter(None, Control, lambda _: Control.none())


class ControlList(Expr):

    def __getitem__(self, index: Num) -> Control: ...

    # Returns the length of this list.
    def length(self) -> Num: ...

    # à la Python: concatenation
    def __add__(self, other: ControlList) -> ControlList: ...


@function
def nil() -> ControlList: ...


@function
def cons(x: Control, xs: ControlList) -> ControlList: ...


converter(tuple, ControlList, lambda t: nil() if not t else cons(t[0], t[1:]))
converter(list, ControlList, lambda t: nil() if not t else cons(t[0], t[1:]))


@overload
def try_evaling(expr: Expr, prim_expr: i64) -> int: ...


@overload
def try_evaling(expr: Expr, prim_expr: Bool) -> bool: ...


def try_evaling(expr: Expr, prim_expr: i64 | Bool) -> int | bool:
    """
    Try evaling the expression, and if it fails, display the egraph and raise an error.
    """
    egraph = EGraph.current()
    egraph.register(expr)
    egraph.run(calyx_ruleset)
    try:
        return egraph.eval(prim_expr)
    except EggSmolError as exc:
        egraph.display(n_inline_leaves=2, split_primitive_outputs=True)
        msg = "Cannot simplify:"
        raise ValueError(msg, egraph.extract(expr)) from exc
