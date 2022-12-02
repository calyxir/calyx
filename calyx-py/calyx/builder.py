import threading
from typing import Dict, Union, Optional, List
from . import py_ast as ast

# Thread-local storage to keep track of the current GroupBuilder we have
# entered as a context manager. This is weird magic!
TLS = threading.local()
TLS.groups = []


class Builder:
    """The entry-point builder for top-level Calyx programs."""

    def __init__(self):
        self.program = ast.Program(
            imports=[
                ast.Import("primitives/core.futil"),
                ast.Import("primitives/binary_operators.futil"),
                ast.Import("primitives/memories.futil"),
            ],
            components=[],
        )

    def component(self, name: str, cells=[]):
        comp_builder = ComponentBuilder(name, cells)
        self.program.components.append(comp_builder.component)
        return comp_builder


class ComponentBuilder:
    """Builds Calyx components definitions."""

    def __init__(self, name: str, cells: List[ast.Cell] = []):
        """Contructs a new component in the current program. If `cells` is
        provided, the component will be initialized with those cells."""
        self.component: ast.Component = ast.Component(
            name,
            inputs=[],
            outputs=[],
            structs=cells,
            controls=ast.Empty(),
        )
        self.index: Dict[str, Union[GroupBuilder, CellBuilder]] = {}
        for cell in cells:
            self.index[cell.id.name] = CellBuilder(cell)
        self.continuous = GroupBuilder(None, self)

    def input(self, name: str, size: int):
        self.component.inputs.append(ast.PortDef(ast.CompVar(name), size))

    def output(self, name: str, size: int):
        self.component.outputs.append(ast.PortDef(ast.CompVar(name), size))

    def this(self) -> "ThisBuilder":
        return ThisBuilder()

    @property
    def control(self):
        return ControlBuilder(self.component.controls)

    @control.setter
    def control(self, builder: Union[ast.Control, "ControlBuilder"]):
        if isinstance(builder, ControlBuilder):
            self.component.controls = builder.stmt
        else:
            self.component.controls = builder

    def get_cell(self, name: str) -> "CellBuilder":
        out = self.index.get(name)
        if out and isinstance(out, CellBuilder):
            return out
        else:
            raise Exception(
                f"Cell `{name}' not found in component {self.component.name}.\n"
                f"Known cells: {list(map(lambda c: c.id.name, self.component.cells))}"
            )

    def get_group(self, name: str) -> "GroupBuilder":
        out = self.index.get(name)
        if out and isinstance(out, GroupBuilder):
            return out
        else:
            raise Exception(
                f"Group `{name}' not found in component {self.component.name}"
            )

    def group(self, name: str) -> "GroupBuilder":
        group = ast.Group(ast.CompVar(name), connections=[])
        self.component.wires.append(group)
        builder = GroupBuilder(group, self)
        self.index[name] = builder
        return builder

    def comb_group(self, name: str) -> "GroupBuilder":
        group = ast.CombGroup(ast.CompVar(name), connections=[])
        self.component.wires.append(group)
        builder = GroupBuilder(group, self)
        self.index[name] = builder
        return builder

    def cell(
        self, name: str, comp: ast.CompInst, is_external=False, is_ref=False
    ) -> "CellBuilder":
        cell = ast.Cell(ast.CompVar(name), comp, is_external, is_ref)
        self.component.cells.append(cell)
        builder = CellBuilder(cell)
        self.index[name] = builder
        return builder

    def reg(self, name: str, size: int):
        stdlib = ast.Stdlib()  # TODO Silly; should be @staticmethods.
        return self.cell(name, stdlib.register(size))

    def add(self, name: str, size: int):
        stdlib = ast.Stdlib()
        return self.cell(name, stdlib.op("add", size, signed=False))


def as_control(obj):
    """Convert a Python object into a control statement.

    This is the machinery for treating shorthand Python expressions as
    control statements. The rules are:

    * Strings are "enable" leaves.
    * Groups (and builders) are also enables.
    * Lists are `seq` statements.
    * `None` is the `empty` statement.
    * Otherwise, this should be an actual `Control` value.
    """
    if isinstance(obj, ast.Control):
        return obj
    elif isinstance(obj, str):
        return ast.Enable(obj)
    elif isinstance(obj, ast.Group):
        return ast.Enable(obj.id.name)
    elif isinstance(obj, GroupBuilder):
        return ast.Enable(obj.group.id.name)
    elif isinstance(obj, list):
        return ast.SeqComp([as_control(o) for o in obj])
    elif isinstance(obj, set):
        return ast.ParComp([as_control(o) for o in obj])
    elif obj is None:
        return ast.Empty()
    else:
        assert False, f"unsupported control type {type(obj)}"


def while_(port: "ExprBuilder", cond: "GroupBuilder", body):
    """Build a `while` control statement."""
    return ast.While(port.expr, cond.group.id, as_control(body))


class ControlBuilder:
    """Wraps control statements for convenient construction."""

    def __init__(self, stmt=None):
        self.stmt = as_control(stmt)

    def __add__(self, other):
        """Build sequential composition."""
        other_stmt = as_control(other)

        if isinstance(self.stmt, ast.Empty):
            # Special cases for when one side is empty.
            return ControlBuilder(other_stmt)
        elif isinstance(other_stmt, ast.Empty):
            return self
        elif isinstance(self.stmt, ast.SeqComp) and isinstance(other_stmt, ast.SeqComp):
            # Special cases for when we already have at least one seq.
            return ControlBuilder(ast.SeqComp(self.stmt.stmts + other_stmt.stmts))
        elif isinstance(self.stmt, ast.SeqComp):
            return ControlBuilder(ast.SeqComp(self.stmt.stmts + [other_stmt]))
        elif isinstance(other_stmt, ast.SeqComp):
            return ControlBuilder(ast.SeqComp([self.stmt] + other_stmt.stmts))
        else:
            # General case.
            return ControlBuilder(ast.SeqComp([self.stmt, as_control(other)]))


class ExprBuilder:
    """Wraps an assignment expression.

    This wrapper provides convenient ways to build logical expressions
    for guards. Use the Python operators &, |, and ~ to build and, or,
    and not expressions in Calyx.

    It also provides an @ operator to help build conditional
    expressions, by creating a `CondExprBuilder`.
    """

    def __init__(self, expr: ast.GuardExpr):
        self.expr = expr

    def __and__(self, other: "ExprBuilder"):
        """Construct an "and" logical expression with &."""
        return ExprBuilder(ast.And(self.expr, other.expr))

    def __or__(self, other: "ExprBuilder"):
        """Construct an "or" logical expression with |."""
        return ExprBuilder(ast.Or(self.expr, other.expr))

    def __invert__(self):
        """Construct a "not" logical expression with ~."""
        return ExprBuilder(ast.Not(self.expr))

    def __matmul__(self, rhs: "ExprBuilder"):
        """Construct a conditional expression with @.

        This produces a `CondExprBuilder`, which wraps a value
        expression and a logical condition expression. Assigning this
        into any port or hole *makes that assignment conditional*. It is
        not possible to compose larger expressions out of
        `CondExprBuilder` values; they are only useful for assignment.
        """
        return CondExprBuilder(self, rhs)

    @classmethod
    def unwrap(cls, obj):
        if isinstance(obj, cls):
            return obj.expr
        else:
            return obj


class CondExprBuilder:
    def __init__(self, cond, value):
        self.cond = cond
        self.value = value


class CellLikeBuilder:
    """Wraps a cell-like for convenient wire building.

    When we're in the context for a group builder (inside a `with
    group:` block), use `cell.port = expr` or `cell["port"] = expr` to
    add an assignment to the active group.

    Using "dot syntax" works only for port names that *do not* begin
    with a `_` (underscore fields are reserved for internal operations).
    To avoid collisions with Python reserved words, you can use an
    underscore suffix: for example, use `reg.in_ = ...`. When in doubt,
    you can always use subscripting to provide an explicit string.
    """

    def __init__(self):
        pass

    def port(self, name: str):
        raise NotImplementedError()

    def __getitem__(self, key):
        return self.port(key)

    def __getattr__(self, key):
        if key.startswith("_"):
            return object.__getattr__(self, key)
        elif key.endswith("_"):
            return self.port(key[:-1])
        else:
            return self.port(key)

    def ctx_asgn(self, port: str, rhs: ExprBuilder):
        """Add an assignment to the current group context."""
        ctx_asgn(self.port(port), rhs)

    def __setitem__(self, key, value):
        ctx_asgn(self.port(key), value)

    def __setattr__(self, key, value):
        if key.startswith("_"):
            object.__setattr__(self, key, value)
        elif key.endswith("_"):
            self.ctx_asgn(key[:-1], value)
        else:
            self.ctx_asgn(key, value)


class CellBuilder(CellLikeBuilder):
    """Wraps a cell for convenient wire building."""

    def __init__(self, cell: ast.Cell):
        self._cell = cell

    def port(self, name: str):
        """Build a port access expression."""
        return ExprBuilder(ast.Atom(ast.CompPort(self._cell.id, name)))


class ThisBuilder(CellLikeBuilder):
    """Wraps a component for convenient wire building."""

    def __init__(self):
        pass

    def port(self, name: str):
        """Build a port access expression."""
        return ExprBuilder(ast.Atom(ast.ThisPort(ast.CompVar(name))))


class GroupBuilder:
    """Wraps a group for easy addition of assignment statements.

    The basic mechanism here is the `asgn` method, which builds a single
    assignment statement and adds it to the underlying group. The `done`
    property also provides access to the group's done hole.

    There is also a fancy, magical way to add assignments using Python
    assignment syntax based on Python's context managers. Use `with` on
    a `GroupBuilder` to enter a context where assignments can
    *implicitly* get added to this group.
    """

    def __init__(self, group: Optional[ast.Group | ast.CombGroup],
                 comp: ComponentBuilder):
        self.group = group
        self.comp = comp

    def asgn(
        self,
        lhs: ExprBuilder,
        rhs: Union[ExprBuilder, CondExprBuilder, int],
        cond: Optional[ExprBuilder] = None,
    ):
        """Add a connection to the group.

        If the assigned value is an int, try to infer a width for it and
        promote it to a constant expression. If it's a `CondExprBuilder`
        (which you create as (`cond @ value`), use the condition
        contained therein for the assignment.
        """

        if isinstance(rhs, CondExprBuilder):
            assert cond is None
            cond = rhs.cond
            rhs = rhs.value

        if isinstance(rhs, int):
            width = infer_width(lhs)
            if not width:
                raise Exception(f"could not infer width for literal {rhs}")
            rhs = const(width, rhs)

        wire = ast.Connect(
            ExprBuilder.unwrap(lhs),
            ExprBuilder.unwrap(rhs),
            ExprBuilder.unwrap(cond),
        )
        if self.group:
            self.group.connections.append(wire)
        else:
            self.comp.component.wires.append(wire)

    @property
    def done(self):
        """The `done` hole for the group."""
        assert self.group, (
            "GroupBuilder represents continuous assignments"
            " and does not have a done hole"
        )
        return ExprBuilder(ast.HolePort(ast.CompVar(self.group.id.name),
                                        "done"))

    @done.setter
    def done(self, expr):
        """Build an assignment to `done` in the group."""
        if not self.group:
            raise Exception(
                "GroupBuilder represents continuous assignments and does not have a done hole"
            )
        self.asgn(self.done, expr)

    def __enter__(self):
        TLS.groups.append(self)
        return self

    def __exit__(self, exc, value, tb):
        TLS.groups.pop()


def const(width: int, value: int):
    """Build a sized integer constant expression.

    This is available as a shorthand in cases where automatic width
    inference fails. Otherwise, you can just use plain Python integer
    values.
    """
    return ast.ConstantPort(width, value)


def infer_width(expr):
    """Try to guess the width of a port expression.

    Return an int, or None if we don't have a guess.
    """
    assert TLS.groups, "int width inference only works inside `with group:`"
    group_builder: GroupBuilder = TLS.groups[-1]

    # Deal with `done` holes.
    expr = ExprBuilder.unwrap(expr)
    if isinstance(expr, ast.HolePort):
        assert expr.name == "done", f"unknown hole {expr.name}"
        return 1

    # Otherwise, it's a `cell.port` lookup.
    assert isinstance(expr, ast.Atom)
    cell_name = expr.item.id.name
    port_name = expr.item.name

    # Look up the component for the referenced cell.
    cell_builder = group_builder.comp.index[cell_name]
    if isinstance(cell_builder, CellBuilder):
        inst = cell_builder._cell.comp
    else:
        return None

    # Extract widths from stdlib components we know.
    prim = inst.id
    if prim == "std_reg":
        if port_name == "in":
            return inst.args[0]
        elif port_name == "write_en":
            return 1
    elif prim == "std_add":
        if port_name == "left" or port_name == "right":
            return inst.args[0]
    elif prim == "std_mem_d1" or prim == "seq_mem_d1":
        if port_name == "write_en":
            return 1
        elif port_name == "addr0":
            return inst.args[2]
        elif port_name == "in":
            return inst.args[0]
        if prim == "seq_mem_d1":
            if port_name == "read_en":
                return 1
    elif (
        prim == "std_mult_pipe"
        or prim == "std_smult_pipe"
        or prim == "std_mod_pipe"
        or prim == "std_smod_pipe"
        or prim == "std_div_pipe"
        or prim == "std_sdiv_pipe"
    ):
        if port_name == "left" or port_name == "right":
            return inst.args[0]
        elif port_name == "go":
            return 1

    # Give up.
    return None


def ctx_asgn(lhs: ExprBuilder, rhs: Union[ExprBuilder, CondExprBuilder]):
    """Add an assignment to the current group context."""
    assert TLS.groups, "assignment outside `with group`"
    group_builder: GroupBuilder = TLS.groups[-1]
    group_builder.asgn(lhs, rhs)
