from __future__ import annotations
from egglog.bindings import *
from typing import Sequence
from egglog import *


class Node(Expr):
    """
    Node is a i64 or a (deeply nested) list of nodes.
    e.g.,
    0 ;; [0, 1, 2] ;; [[0, 1], 2] ;; [[][[][[1]]]]
    """

    # Returns the length of this structure, to include nested structures.
    def glength(self) -> i64: ...

    def length(self) -> i64: ...

    def __init__(self) -> None: ...


class List(Expr):
    # Returns the length of this list and all sublists.
    def glength(self) -> i64: ...

    def length(self) -> i64: ...
    def maxl(self) -> i64: ...

    # Concatenation.
    def __add__(self, other: List) -> List: ...

    # Slice.
    def _sliceE(self, end: i64) -> List: ...
    def _sliceB(self, begin: i64) -> List: ...
    def slice(self, begin: i64, end: i64) -> List: ...

    # Reverse.
    def rev(self) -> List: ...

    # Contains.
    def contains(self, node: Node) -> Bool: ...


@function
def nil() -> List: ...
@function
def cons(x: Node, xs: List) -> List: ...


@function
def Int(n: i64) -> Node: ...
@function
def Slist(list: List) -> Node: ...
@function
def SMap(map: Map[String, i64]) -> Node: ...


@function
def _max(a: i64, b: i64) -> i64: ...


converter(int, Node, lambda t: Int(t))
converter(list, List, lambda t: nil() if not t else cons(t[0], t[1:]))

egraph = EGraph()


@egraph.register
def _(r: List, xs: List, ys: List, xss: List, x: Node, y: Node, i: i64, j: i64):
    """
    List Length
    """
#    yield rewrite(nil().length()).to(i64(0))
#    yield rewrite(cons(x, xs).length()).to(i64(1) + xs.length())
    yield rule(
        eq(r).to(nil())
    ).then(
        set_(r.length()).to(i64(0))
    )
    yield rule(
        eq(r).to(cons(x, xs)),
        xs.length()  # Must "demand" the length of `xs` in the facts.
    ).then(
        set_(r.length()).to(xs.length() + i64(1))
    )

    """
    List Max
    """
    yield rule(
        eq(r).to(nil())
    ).then(
        set_(r.maxl()).to(i64(0))
    )

    yield rule(
        eq(r).to(cons(Int(i), xs)),
        xs.maxl(),
        _max(i, xs.maxl()),
    ).then(
        set_(r.maxl()).to(_max(i, xs.maxl()))
    )
    yield rule(
        eq(r).to(cons(Slist(ys), xs)),
        xs.maxl(),
    ).then(
        set_(r.maxl()).to(xs.maxl())
    )

    yield rewrite(
        _max(i, j)
    ).to(
        i, i > j
    )
    yield rewrite(
        _max(i, j),
    ).to(
        j, i <= j
    )

    # Base case
    yield rule(
        eq(r).to(nil())
    ).then(
        set_(r.glength()).to(i64(0))
    )

    # Inductive case
    yield rule(
        eq(r).to(cons(Int(i), xs)),
        xs.glength()  # Must "demand" the length of `xs` in the facts.
    ).then(
        set_(r.glength()).to(xs.glength() + i64(1))
    )
    yield rule(
        eq(r).to(cons(Slist(ys), xs)),
        xs.glength(), ys.glength(),
    ).then(
        set_(r.glength()).to(xs.glength() + ys.glength())
    )

    """List Contains"""
    # rel = relation("x", Node)
    # yield rule(
    #     eq(r).to(nil()),
    #     rel(x),
    # ).then(
    #     set_(r.contains(x)).to(Bool(False))
    # )

    # yield rule(
    #     eq(r).to(cons(x, xs)),
    # ).then(
    #     set_(r.contains(x)).to(Bool(True))
    # )

    # yield rule(
    #     eq(r).to(cons(x, xs)),
    #     xs.contains(y),
    # ).then(
    #     set_(r.contains(y)).to(xs.contains(y))
    # )
    # TODO(cgyurgyik): contains does not work :(
    yield rewrite(nil().contains(x)).to(Bool(False))
    yield rewrite(cons(x, xs).contains(x)).to(Bool(True))
    yield rewrite(cons(x, xs).contains(y)).to(xs.contains(y))


path = relation("path", Node, Node)  # Directed path
edge = relation("edge", Node, Node)  # Directed edge


@egraph.register
def _(x: Node, y: Node, z: Node):
    yield rule(edge(x, y)).then(path(x, y))
    yield rule(path(x, y), edge(y, z)).then(path(x, z))


@egraph.register
def _(x: Node, y: Node, r: List, xs: List, ys: List, begin: i64, end: i64):
    """
    List Reverse
    """
    yield rule(eq(xs).to(nil())).then(set_(xs.rev()).to(nil()))
    yield rule(
        eq(ys).to(cons(x, xs)),
        xs.rev()  # Demand
    ).then(
        set_(ys.rev()).to(xs.rev() + cons(x, nil()))
    )

    """
    List Concatenation
    """
    yield rewrite(cons(x, xs) + nil()).to(cons(x, xs))
    yield rewrite(nil() + cons(x, xs)).to(cons(x, xs))
    yield rewrite(cons(x, nil()) + cons(y, ys)).to(cons(x, cons(y, ys)))
    yield rewrite(cons(x, xs) + ys).to(cons(x, xs + ys))

    """
    List Slice
    """
    yield rewrite(
        cons(x, xs)._sliceE(0)
    ).to(
        nil()
    )
    yield rewrite(
        cons(x, xs)._sliceE(end)
    ).to(
        cons(x, xs._sliceE(end - 1)),
        end > 0,
        end <= xs.length(),
    )

    yield rewrite(
        cons(x, xs)._sliceE(end)
    ).to(
        cons(x, xs),
        eq(end).to(cons(x, xs).length())
    )

    yield rewrite(
        cons(x, xs)._sliceB(0)
    ).to(
        cons(x, xs)
    )

    yield rewrite(
        cons(x, xs)._sliceB(begin)
    ).to(
        xs._sliceB(begin - 1),
        begin > 0
    )

    yield rewrite(
        cons(x, xs).slice(begin, end)
    ).to(
        cons(x, xs)._sliceB(begin)._sliceE(end - begin),
        end >= begin
    )

    # par { a; b; c; d; } => par { par { a; b; } par { c; d; } }
    N = 4
    yield rewrite(
        Slist(xs),
    ).to(
        Slist([
            Slist(xs.slice(0, xs.length() / 2)),
            Slist(xs.slice(xs.length() / 2, xs.length()))
        ]),
        xs.length() >= N,
        # TODO(cgyurgyik): fail(path(x, y)) ?
    )


# l1 = Slist(cons(1, cons(2, cons(3, cons(4, nil())))).slice(2, 4))
# l1_1 = Slist(cons(1, cons(2, cons(3, cons(4, nil())))).slice(1, 3))
# l1_2 = Slist(cons(1, cons(2, cons(3, cons(4, nil())))))
# l1_3 = Slist((cons(1, cons(2, nil()) + cons(3, cons(4, nil())))))
# l2 = Slist(cons(3, cons(4, nil())).rev())
# l3 = Slist(cons(1, cons(2, cons(3, nil())))._sliceE(2))
# l4 = Slist(cons(3, cons(2, cons(1, nil())))._sliceE(1))
# l5 = Slist(cons(3, cons(2, cons(1, nil())))._sliceB(1))
# l6 = Slist(cons(Int(1), cons(Int(2), cons(Int(3), cons(Int(4), nil())))))

# b1 = nil()  # cons(1, cons(2, cons(3, cons(4, nil()))))
# r1 = b1.contains(Int(2))

# egraph.let("l1", l1)
# egraph.let("l1_1", l1_1)
# egraph.let("l1_2", l1_2)
# egraph.let("l1_3", l1_3)
# egraph.let("l2", l2)
# egraph.let("l3", l3)
# egraph.let("l4", l4)
# egraph.let("l5", l5)
# egraph.let("l6", l6)


x0 = Int(1)
y0 = Int(2)
a = Int(5)
b = Int(6)
x1 = Int(3)
y1 = Int(4)

x0_a = egraph.let("x0_a", edge(x0, a))
x0_b = egraph.let("x0_b", edge(x0, b))
y0_a = egraph.let("y0_a", edge(y0, a))
y0_b = egraph.let("y0_b", edge(y0, b))
a_y1 = egraph.let("a_y1", edge(a, y1))
a_x1 = egraph.let("a_x1", edge(a, x1))
b_y1 = egraph.let("b_y1", edge(b, y1))
b_x1 = egraph.let("b_x1", edge(b, x1))

L = egraph.let("L", Slist([Slist([x0, y0]), a, b, Slist([x1, y1])]))
L = egraph.let("m", Int(_max(i64(1), i64(0))))

L = egraph.simplify(L, 32)
egraph.display()

# egraph.check(path(a, b))
# egraph.check(eq(L).to(Slist([Slist([x0, y0]), Slist([a, b]), Slist([x1, y1])])))
# egraph.check(
#     eq(l3).to(Slist(cons(1, cons(2, nil()))))
# )

# egraph.check(
#     eq(l4).to(Slist(cons(3, nil())))
# )

# egraph.check(
#     eq(l5).to(Slist(cons(2, cons(1, nil()))))
# )

# egraph.display()

# egraph.check(
#     eq(l1).to(Slist(cons(3, cons(4, nil()))))
# )

# egraph.check(
#     eq(l1_1).to(Slist(cons(2, cons(3, nil()))))
# )


# egraph.check(
#     eq(l1_3).to(Slist([Slist([1, 2]), Slist([3, 4])]))
# )

# egraph.check(eq(r1).to(Bool(False)))
