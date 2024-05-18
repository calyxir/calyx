from __future__ import annotations
from typing import List, Sequence, Union, Optional, TypeAlias, TypeVar, Any
import calyx.py_ast as ast
import egg_ast as egg
import egg_ruleset
from egglog import *


class CalyxEgg():
    id: int = 0
    egraph: EGraph = EGraph()
    schedule = (
        egg_ruleset.resource_sharing
        | egg_ruleset.lists
        | egg_ruleset.control
    ).saturate()

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
        expr = self._let(expr)
        self.run()
        return expr

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
                raise NotImplementedError("unexpected")
                # return self.ConvertControl(node)
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

    def ConvertComponent(self, component: ast.Component) -> egg.Control:
        names: Sequence[str] = [g.id.name for g in component.wires]
        groups = {id: group for (id, group) in zip(names, component.wires)}
        assert (all(isinstance(g, ast.Group) for g in groups.values()))
        return self.ConvertControl(component.controls, component, groups)

    def ConvertCompVar(self, var: ast.CompVar) -> egg.CompVar:
        return egg.CompVar(
            name=var.name
        )

    def ConvertCompInst(self, comp: ast.CompInst) -> egg.CompInst:
        return egg.CompInst(
            id=comp.id,
            args=Vec[i64](*[i64(i) for i in comp.args])
        )

    # TODO(cgyurgyik): Elaborate.
    def ConvertCell(self, name: str) -> egg.Cell:
        return egg.Cell(name=name)

    def ConvertCells(self, cells: Sequence[ast.Cell]) -> Set[egg.Cell]:
        return Set[egg.Cell](*[self.ConvertCell(c) for c in cells])

    def ConvertPortDef(self, portdef: ast.PortDef) -> egg.PortDef:
        return egg.PortDef(
            id=self.ConvertCompVar(portdef.id),
            width=i64(portdef.width),
        )

    def ConvertPortDefs(self, portdefs: Sequence[ast.PortDef]) -> Vec[egg.PortDef]:
        return Vec[egg.PortDef](*[self.ConvertPortDef(p) for p in portdefs])

    def ConvertPort(self, port: ast.Port) -> egg.Port:
        match port:
            case ast.ConstantPort(width, value):
                return egg.ConstantPort(i64(width), i64(value))
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

    def CellsUsed(self, port) -> Sequence[str]:
        match port:
            case ast.ConstantPort(_, _):
                return set()
            case ast.HolePort(ast.CompVar(id), _):
                return set()
            case ast.CompPort(ast.CompVar(id), _):
                return {id}
            case ast.Atom(item):
                return self.CellsUsed(item)
            case _:
                raise NotImplementedError(f"{port} of type: {type(port)}")

    def ConvertGroup(self, group: ast.Group, component: ast.Component):
        cells = set(x.id.name for x in component.cells)
        incumbent = set()
        for connect in group.connections:
            cells_used = set()
            cells_used |= self.CellsUsed(connect.dest)
            cells_used |= self.CellsUsed(connect.src)
            # TODO(cgyurgyik): Will the guard ever use ports not previously used?
            incumbent |= cells_used
        return egg.Group(
            name=group.id.name,
            cells=self.ConvertCells(set.intersection(incumbent, cells))
        )

    def ConvertGroups(self, groups: Sequence[ast.Group]) -> Vec[egg.Group]:
        raise NotImplementedError("XXX")
        # return Vec[egg.Group](*[self.ConvertGroup(g) for g in groups])

    def ConvertControl(self, control: ast.Control, component: ast.Component, groups: dict[str, ast.Group]) -> egg.Control:
        match control:
            case ast.Enable(name, promotable):
                group: ast.Group = groups[name]
                a = Map[String, i64].empty()
                if promotable is not None:
                    a = a.insert(String("promotable"), i64(promotable))
                elif group.static_delay is not None:
                    a = a.insert(String("promotable"), i64(group.static_delay))

                return egg.Enable(self.ConvertGroup(group, component), egg.Attributes(a))
            case ast.SeqComp(stmts):
                return egg.Seq([self.ConvertControl(s, component, groups) for s in stmts])
            case ast.ParComp(stmts):
                return egg.Par([self.ConvertControl(s, component, groups) for s in stmts])
            case _:
                raise NotImplementedError(f"{control} of type {type(control)}")
