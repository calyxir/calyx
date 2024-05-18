from __future__ import annotations
from collections.abc import Callable
from typing import Generic, ClassVar
from egg_ast import *
from egglog import *

control = ruleset(name="control")  # Calyx AST
resource_sharing = ruleset(name="resource sharing")  # Resource sharing
lists = ruleset(name="lists")  # Lists (commonly used in control flow)


@lists.register
def _(x: Control, y: Control, r: clist, xs: clist, ys: clist, begin: i64, end: i64, g: Group, m: Map[String, i64]):
    # Length
    # yield rewrite(nil().length()).to(i64(0))
    # yield rewrite(cons(x, xs).length()).to(i64(1) + xs.length())
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
    # Concatenation
    for op in (Par, Seq):
        yield rewrite(op(cons(x, xs) + nil())).to(op(cons(x, xs)))
        yield rewrite(op(nil() + cons(x, xs))).to(op(cons(x, xs)))
        yield rewrite(op(cons(x, nil()) + cons(y, ys))).to(op(cons(x, cons(y, ys))))
        yield rewrite(op(cons(x, xs) + ys)).to(op(cons(x, xs + ys)))
    # Slice
    # yield rewrite(cons(x, xs).slice(begin, end).length()).to(end - begin)
    yield rewrite(cons(x, xs)._sliceE(0)).to(nil())
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

    # yield rewrite(nil().max_latency()).to(i64(0))
    yield rewrite(
        cons(Enable(g, Attributes(m)), xs).max_latency()
    ).to(
        m[String("promotable")],
        m.contains(String("promotable")),
        m[String("promotable")] > xs.max_latency()
    )

    yield rewrite(
        cons(Enable(g, Attributes(m)), xs).max_latency()
    ).to(
        # XXX: Maybe define a max function on integers?
        max(m[String("promotable")], xs.max_latency()),
        m.contains(String("promotable")),
        m[String("promotable")] < xs.max_latency()
    )

    yield rewrite(
        Par(xs)
    ).to(
        Par(nil()),
        xs.max_latency() > -1
    )

    yield rule(
        eq(r).to(nil())
    ).then(
        set_(r.max_latency()).to(i64(0))
    )
    # yield rule(
    #     eq(r).to(cons(Enable(g, Attributes(m)), xs)),
    #     xs.max_latency(),  # Must "demand" the length of `xs` in the facts.
    #     m.contains(String("promotable")),
    #     m[String("promotable")]
    # ).then(
    #     set_(r.max_latency()).to(m[String("promotable")]),
    #     m.contains(String("promotable")),
    #     m[String("promotable")] > xs.max_latency()
    # )


@control.register
def _(x: Control, xs: clist, ys: clist, g1: Group, g2: Group, m: Map[String, i64], m1: Map[String, i64], m2: Map[String, i64]):
    N0 = 4
    N1 = 1000
    N2 = 10
    # TODO(cgyurgyik): collapse-control.

    # par { A; B; C; D; } => par { par { A; B; } par { C; D; } }
    assert N0 >= 4, f"received: {N0}"  # Avoid graph blow-up when N == 2.
    yield rewrite(
        Par(xs),
        subsume=True,
    ).to(
        Par([
            Par(xs.slice(0, xs.length() / 2)),
            Par(xs.slice(xs.length() / 2, xs.length()))
        ]),
        xs.length() >= N0,
    )

    # par { @promotable(1000) A; @promotable(1) B; } =>
    # seq { @promotable(1000) A; @promotable(1) B; }
    for op in (lambda x1, x2, y: x1 - x2 >= y, lambda x1, x2, y: x2 - x1 >= y):
        yield rewrite(
            Par(cons(Enable(g1, Attributes(m1)), cons(Enable(g2, Attributes(m2)), nil()))),
            subsume=True,
        ).to(
            Seq(cons(Enable(g1, Attributes(m1)), cons(Enable(g2, Attributes(m2)), nil()))),
            m1.contains(String("promotable")),
            m2.contains(String("promotable")),
            op(m1[String("promotable")], m2[String("promotable")], N1)
        )

    # seq { A0; A1; ...; An; } =>
    # seq {
    #   @new_fsm seq {A0; ...; A(n/2); }
    #   @new_fsm seq {A(n/2); ...; An; }
    # }
    yield rewrite(
        Seq(xs, Attributes(m)),
        subsume=True,
    ).to(
        Seq([
            Seq(xs.slice(0, xs.length() / 2),
                Attributes(Map[String, i64].empty().insert(String("new_fsm"), i64(1)))),
            Seq(xs.slice(xs.length() / 2, xs.length()),
                Attributes(Map[String, i64].empty().insert(String("new_fsm"), i64(1))))
        ]),
        xs.length() >= N2,
        m.not_contains(String("new_fsm")),
    )

    # TODO(cgyurgyik): Need some dependency analysis.
    # @static(22) seq { A (1); B (10); C (1); D (10); }
    #       A -> C, D
    #       B -> C
    # @static(11) par {
    #  A;
    #  B;
    #  static seq {del_10; C; }
    #  static seq {del_1;  D; }
    # }


@resource_sharing.register
def _(x: Control, xs: clist, n1: String, n2: String, cs1: Set[Cell], cs2: Set[Cell], m1: Attributes, m2: Attributes, c1: Cell, c2: Cell):
    return []
    # TODO(cgyurgyik): Eventually we will require a more advanced liveness analysis.
    #  For a given cell pair (c1, c2) in (n1, n2) respectively, we can replace c1 with c2 if:
    # (!= n1  n2)                         ...these are different groups
    # (== (structure c1) (structure c2))  ...both groups use structurally equivalent cells
    # (!= (id c1) (id c2))                ...both cells are not syntactically equivalent

    # yield rewrite(
    #     Seq([Enable(Group(n1, cs1), m1), Enable(Group(n2, cs2), m2)]),
    # ).to(
    #     Seq([Enable(Group(n1, cs1), m1), Enable(Group(n2, cs1), m2)]),  # XXX
    #     # n1 != n2,
    #     # cs1.contains(c1),
    #     # cs2.contains(c2),
    # )
