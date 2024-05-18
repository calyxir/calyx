from __future__ import annotations
from typing import List, Sequence, Union, Optional, TypeAlias, TypeVar, Any
import calyx.py_ast as ast
import egg_ast as egg
from egg_ruleset import calyx_ruleset
from egglog import *

# TODO(cgyurgyik): have different rule sets so we can try different optimizations together.


class CalyxEgg():
    id: int = 0
    egraph: EGraph = EGraph()
    schedule = calyx_ruleset.saturate()

    def display(self) -> None:
        return self.egraph.display()

    def run(self) -> None:
        return self.egraph.run(self.schedule)

    def simplify(self, expr: Expr) -> Expr:
        return self.egraph.simplify(self._let(expr), self.schedule)

    def extract_multiple(self, expr: Expr, n: int) -> Sequence[Expr]:
        """
        Extracts multiple versions of `expr`.
        """
        self.run()
        return self.egraph.extract_multiple(expr, n)

    def extract(self, expr: Expr) -> Expr:
        self.run()
        return self.egraph.extract(expr)

    def _let(self, expr: Expr, name: Optional[str] = None) -> Expr:
        def _uniq(name: Optional[str] = None):
            name = f"%{self.id}" if name is None else f"{name}{self.id}"
            self.id += 1
            return name

        return self.egraph.let(_uniq(name), expr)

    def _Convert(self, node):
        match node:
            case ast.Component():
                return self.ConvertComponent(node)
            case ast.Control():
                return self.ConvertControl(node)
            case ast.Group():
                return self.ConvertGroup(node)
            case ast.PortDef():
                return self.ConvertPortDef(node)
            case ast.CompVar():
                return self.ConvertCompVar(node)
            case ast.CompInst():
                return self.ConvertCompInst(node)
            case ast.Connect():
                return self.ConvertConnect(node)
            case ast.Cell():
                return self.ConvertCell(node)
            case ast.Port():
                return self.ConvertPort(node)
            case ast.GuardExpr():
                return self.ConvertGuard(node)
            case _:
                raise NotImplementedError(f"unexpected: {node} of type: {type(node)}")

    def Convert(self, node):
        return self._Convert(node)

    def ConvertComponent(self, component: ast.Component) -> egg.Component:
        return egg.Component(
            name=component.name,
            inputs=self.ConvertPortDefs(component.inputs),
            outputs=self.ConvertPortDefs(component.outputs),
            cells=self.ConvertCells(component.cells),
            wires=self.ConvertGroups(component.wires),
            controls=self.ConvertControl(component.controls)
        )

    def ConvertCompVar(self, var: ast.CompVar) -> egg.CompVar:
        return egg.CompVar(
            name=var.name
        )

    def ConvertCompInst(self, comp: ast.CompInst) -> egg.CompInst:
        return egg.CompInst(
            id=comp.id,
            args=Vec[egg.Num](*[egg.Num(i) for i in comp.args])
        )

    def ConvertCell(self, cell: ast.Cell) -> egg.Cell:
        return egg.Cell(
            id=self.ConvertCompVar(cell.id),
            comp=self.ConvertCompInst(cell.comp)
        )

    def ConvertCells(self, cells: Sequence[ast.Cell]) -> Vec[egg.Cell]:
        return Vec[egg.Cell](*[self.ConvertCell(c) for c in cells])

    def ConvertPortDef(self, portdef: ast.PortDef) -> egg.PortDef:
        return egg.PortDef(
            id=self.ConvertCompVar(portdef.id),
            width=egg.Num(portdef.width),
        )

    def ConvertPortDefs(self, portdefs: Sequence[ast.PortDef]) -> Vec[egg.PortDef]:
        return Vec[egg.PortDef](*[self.ConvertPortDef(p) for p in portdefs])

    def ConvertPort(self, port: ast.Port) -> egg.Port:
        match port:
            case ast.ConstantPort(width, value):
                return egg.ConstantPort(egg.Num(width), egg.Num(value))
            case ast.HolePort(id, name):
                return egg.HolePort(self.ConvertCompVar(id), name)
            case ast.CompPort(id, name):
                return egg.CompPort(self.ConvertCompVar(id), name)
            case ast.Atom(item):
                return self.ConvertPort(item)
            case _:
                raise NotImplementedError(f"{port} of type: {type(port)}")

    def ConvertGuard(self, guard: ast.GuardExpr) -> egg.GuardExpr:
        match guard:
            case ast.Atom(item):
                return egg.Atom(self.ConvertPort(item))
            case ast.Not(g):
                return egg.Not(self.ConvertGuard(g))
            case ast.And(lhs, rhs):
                return egg.And(self.ConvertGuard(lhs), self.ConvertGuard(rhs))
            case ast.Or(lhs, rhs):
                return egg.Or(self.ConvertGuard(lhs), self.ConvertGuard(rhs))
            case None:
                return egg.GuardExpr.none()
            case _:
                raise NotImplementedError(f"{guard} of type: {type(guard)}")

    def ConvertConnect(self, connect: ast.Connect) -> egg.Connect:
        return egg.Connect(
            dest=self.ConvertPort(connect.dest),
            src=self.ConvertPort(connect.src),
            guard=self.ConvertGuard(connect.guard)
        )

    def ConvertConnections(self, connects: Sequence[ast.Connect]) -> Vec[egg.Connect]:
        return Vec[egg.Connect](*[self.ConvertConnect(c) for c in connects])

    def ConvertGroup(self, group: ast.Group):
        delay: Optional[int] = group.static_delay
        return egg.Group(
            name=self.ConvertCompVar(group.id),
            connects=self.ConvertConnections(group.connections),
            promotable=egg.OptionalNum.none() if delay is None else delay,
        )

    def ConvertGroups(self, groups: Sequence[ast.Group]) -> Vec[egg.Group]:
        return Vec[egg.Group](*[self.ConvertGroup(g) for g in groups])

    def ConvertControl(self, control: ast.Control) -> egg.Control:
        match control:
            case ast.Enable(name, promotable):
                return egg.Enable(name, egg.OptionalNum.none() if promotable is None else promotable)
            case ast.SeqComp(stmts):
                return egg.Seq([self.ConvertControl(s) for s in stmts])
            case ast.ParComp(stmts):
                return egg.Par([self.ConvertControl(s) for s in stmts])
            case _:
                raise NotImplementedError(f"{control} of type {type(control)}")
