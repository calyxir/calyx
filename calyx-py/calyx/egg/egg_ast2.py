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


############################################################
########################## Calyx ###########################
############################################################

class CompInst(Expr):
    def __init__(self, id: StringLike, args: Vec[Num]) -> None: ...

    def __eq__(self, other: CompInst) -> Boolean: ...


class Cell(Expr):
    def __init__(self, id: CompVar, comp: CompInst) -> None: ...

class CompVar(Expr):
    def __init__(self, name: StringLike) -> None: ...


class Group(Expr):
    # TODO(cgyurgyik): Getting weird error when using Set.
    def __init__(self, name: StringLike, cells: Vec[StringLike]) -> None: ...


class Control(Expr):
    def __init__(self) -> None: ...

    @classmethod
    def none(cls) -> Control: ...


@function
def Enable(group: Group, promotable: OptionalNum = OptionalNum.none()) -> Control: ...


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
