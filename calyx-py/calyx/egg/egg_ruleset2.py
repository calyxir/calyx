from __future__ import annotations
from collections.abc import Callable
from typing import Generic, ClassVar
from egg_ast2 import *
from egglog import *

calyx_ruleset = ruleset(name="calyx_ruleset")


@calyx_ruleset.register
def _number(r: Boolean, x: Boolean, i: i64, j: i64):
    # (Num, Boolean) push-down
    yield rewrite(Num(i) + Num(j)).to(Num(i + j))
    yield rewrite(Num(i) - Num(j)).to(Num(i - j))
    yield rule(eq(x).to(TRUE)).then(set_(x.bool).to(Bool(True)))
    yield rule(eq(x).to(FALSE)).then(set_(x.bool).to(Bool(False)))
    yield rewrite(TRUE | x).to(TRUE)
    yield rewrite(FALSE | x).to(x)
    yield rewrite(TRUE & x).to(x)
    yield rewrite(FALSE & x).to(FALSE)

    yield rewrite(Num(i) == Num(i)).to(TRUE)
    yield rule(eq(r).to(Num(i) == Num(j)), ne(i).to(j)).then(union(r).with_(FALSE))

    yield rewrite(Num(i) >= Num(i)).to(TRUE)
    yield rule(eq(r).to(Num(i) >= Num(j)), i > j).then(union(r).with_(TRUE))
    yield rule(eq(r).to(Num(i) >= Num(j)), i < j).then(union(r).with_(FALSE))

    yield rewrite(Num(i) < Num(i)).to(FALSE)
    yield rule(eq(r).to(Num(i) < Num(j)), i < j).then(union(r).with_(TRUE))
    yield rule(eq(r).to(Num(i) < Num(j)), i > j).then(union(r).with_(FALSE))

    yield rewrite(Num(i) > Num(i)).to(FALSE)
    yield rule(eq(r).to(Num(i) > Num(j)), i > j).then(union(r).with_(TRUE))
    yield rule(eq(r).to(Num(i) > Num(j)), i < j).then(union(r).with_(FALSE))


@calyx_ruleset.register
def _list(xs: ControlList, ys: ControlList, x: Control, y: Control, i: i64, j: i64):
    # (ControlList) Indexing
    yield rewrite(cons(x, xs)[Num(0)]).to(x)
    yield rewrite(cons(x, xs)[Num(i)]).to(xs[i - Num(1)], i > i64(0))
    # (ControlList) Length
    yield rewrite(nil().length()).to(Num(0))
    yield rewrite(cons(x, xs).length()).to(Num(1) + xs.length())
    # (ControlList) Concatenation
    # ; TODO(cgyurgyik): e-graph blowup.
    yield rewrite(cons(x, xs) + nil()).to(cons(x, xs))
    yield rewrite(nil() + cons(x, xs)).to(cons(x, xs))
    # yield rewrite(cons(x, nil()) + cons(y, ys)).to(cons(x, cons(y, ys)))
    # yield rewrite(cons(x, xs) + ys).to(cons(x, xs + ys))


@calyx_ruleset.register
def _collapse_empty(l: ControlList, x: Control):
    for C in (Par, Seq):
        # Seq { Seq {} Seq { a; b; c; } } => Seq { a; b; c; }
        yield rewrite(cons(C([]), l)).to(l)
        # Seq { Seq { a; } Seq {} } => Seq { a; }
        yield rewrite(cons(x, cons(C([]), l))).to(cons(x, l))


# @calyx_ruleset.register
# def _join(l1: ControlList, l2: ControlList, l3: ControlList):
#     for C in (Par, Seq):
#         # Seq { Seq { a; } b; } => Seq { a; b; }
#         yield birewrite(C(cons(C(l1), l2))).to(C(l1 + l2))
#         # Seq { Seq { a; } Seq { b; } ...} => Seq { Seq { a; b; } ... }
#         yield birewrite(C(cons(C(l1), cons(C(l2), l3)))).to(C(cons(C(l1 + l2), l3)))

@calyx_ruleset.register
def _fsm_optimization(g1: Group, g2: Group, l1: i64, l2: i64):
    # par { a0; ... ai; ...; an; }
    # seq { ai; par {a0; ...; an; } }
    # ...if latency(ai) > N and latency(a0...an - ai) < epsilon.
    N = i64(1000)
    # TODO(cgyurgyik): Why doesn't | operator work here?
    yield rewrite(Par([Enable(g1, Num(l1)), Enable(g2, Num(l2))])).to(Seq([Enable(g1, Num(l1)), Enable(g2, Num(l2))]), l1 - l2 > N)
    yield rewrite(Par([Enable(g1, Num(l1)), Enable(g2, Num(l2))])).to(Seq([Enable(g1, Num(l1)), Enable(g2, Num(l2))]), l2 - l1 > N)


@calyx_ruleset.register
def _(a: Control, b: Control, c: Control, d: Control):
    yield rewrite(
        Seq([a, b, c, d])
    ).to(
        Seq([Seq([a, b]), Seq([c, d])])
    )
