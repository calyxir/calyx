from __future__ import annotations
from typing import overload, Generic, TypeVar
from egglog import *

converter(int, i64, i64)
converter(str, String, String)


class i64Maybe(Expr):
    def __init__(self, x: i64) -> None: ...

    @classmethod
    def none(cls) -> i64Maybe: ...

    def __add__(self, other: i64Maybe) -> i64Maybe: ...
    def __sub__(self, other: i64Maybe) -> i64Maybe: ...


converter(i64, i64Maybe, i64Maybe)
converter(None, i64Maybe, lambda _: i64Maybe.none())


############################################################
########################## Calyx ###########################
############################################################


class Cell(Expr):
    def __init__(self, name: StringLike) -> None: ...


class Group(Expr):
    # TODO(cgyurgyik): Getting weird error when using Set...
    # "Could not find callable ref for call App("set-insert", [14, 5])"
    def __init__(self, name: StringLike, cells: Set[Cell]) -> None: ...


class Control(Expr):
    def __init__(self) -> None: ...

    @classmethod
    def none(cls) -> Control: ...


class Attributes(Expr):
    def __init__(self, m: Map[String, i64] = Map[String, i64].empty()) -> None: ...


@function
def Enable(group: Group, attributes: Attributes = Attributes()) -> Control: ...


@function
def Seq(control: clist, attributes: Attributes = Attributes()) -> Control: ...


@function
def Par(control: clist, attributes: Attributes = Attributes()) -> Control: ...


converter(None, Control, lambda _: Control.none())


class clist(Expr):
    # Returns the length of this list.
    def length(self) -> i64: ...

    # à la Python: concatenation
    def __add__(self, other: clist) -> clist: ...

    def max_latency(self) -> i64: ...

    # Slice, i.e., [begin, end)
    def slice(self, begin: i64, end: i64) -> clist: ...
    def _sliceE(self, end: i64) -> clist: ...
    def _sliceB(self, begin: i64) -> clist: ...


@function
def nil() -> clist: ...
@function
def cons(x: Control, xs: clist) -> clist: ...


converter(tuple, clist, lambda t: nil() if not t else cons(t[0], t[1:]))
converter(list, clist, lambda t: nil() if not t else cons(t[0], t[1:]))
