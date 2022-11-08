import threading
from . import py_ast as ast
from typing import Dict

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
            ],
            components=[],
        )

    def component(self, name: str):
        comp_builder = ComponentBuilder(name)
        self.program.components.append(comp_builder.component)
        return comp_builder


class ComponentBuilder:
    """Builds Calyx components definitions."""

    def __init__(self, name: str):
        self.component = ast.Component(
            name,
            inputs=[],
            outputs=[],
            structs=[],
            controls=ast.Empty(),
        )
        self.index: Dict[str, GroupBuilder | CellBuilder] = {}

    def __getitem__(self, key):
        return self.index[key]

    @property
    def control(self):
        return ControlBuilder(self.component.controls)

    @control.setter
    def control(self, builder):
        self.component.controls = builder.stmt

    def group(self, name: str):
        group = ast.Group(ast.CompVar(name), connections=[])
        self.component.wires.append(group)
        builder = GroupBuilder(group, self)
        self.index[name] = builder
        return builder

    def cell(self, name: str, comp: ast.CompInst):
        cell = ast.Cell(ast.CompVar(name), comp)
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
    elif obj is None:
        return ast.Empty()
    else:
        assert False, f"unsupported control type {type(obj)}"


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
        elif isinstance(self.stmt, ast.SeqComp) and \
                isinstance(other_stmt, ast.SeqComp):
            # Special cases for when we already have at least one seq.
            return ControlBuilder(
                ast.SeqComp(self.stmt.stmts + other_stmt.stmts)
            )
        elif isinstance(self.stmt, ast.SeqComp):
            return ControlBuilder(
                ast.SeqComp(self.stmt.stmts + [other_stmt])
            )
        elif isinstance(other_stmt, ast.SeqComp):
            return ControlBuilder(
                ast.SeqComp([self.stmt] + other_stmt.stmts)
            )
        else:
            # General case.
            return ControlBuilder(
                ast.SeqComp([self.stmt, as_control(other)])
            )


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

    def __and__(self, other: 'ExprBuilder'):
        """Construct an "and" logical expression with &."""
        return ExprBuilder(ast.And(self.expr, other.expr))

    def __or__(self, other: 'ExprBuilder'):
        """Construct an "or" logical expression with |."""
        return ExprBuilder(ast.Or(self.expr, other.expr))

    def __invert__(self, other: 'ExprBuilder'):
        """Construct a "not" logical expression with ^."""
        return ExprBuilder(ast.Not(self.expr))

    def __matmul__(self, other):
        """Construct a conditional expression with @.

        This produces a `CondExprBuilder`, which wraps a value
        expression and a logical condition expression. Assigning this
        into any port or hole *makes that assignment conditional*. It is
        not possible to compose larger expressions out of
        `CondExprBuilder` values; they are only useful for assignment.
        """
        return CondExprBuilder(self, other)

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


class CellBuilder:
    """Wraps a cell for convenient wire building.

    When we're in the context for a group builder (inside a `with
    group:` block), use `cell.port = expr` or `cell["port"] = expr` to
    add an assignment to the active group.

    Using "dot syntax" works only for port names that *do not* begin
    with a `_` (underscore fields are reserved for internal operations).
    To avoid collisions with Python reserved words, you can use an
    underscore suffix: for example, use `reg.in_ = ...`. When in doubt,
    you can always use subscripting to provide an explicit string.
    """

    def __init__(self, cell: ast.Cell):
        self._cell = cell

    def port(self, name: str):
        """Build a port access expression."""
        return ExprBuilder(ast.Atom(ast.CompPort(self._cell.id, name)))

    def __getitem__(self, key):
        return self.port(key)

    def __getattr__(self, key):
        if key.startswith('_'):
            return object.__getattr__(self, key)
        elif key.endswith('_'):
            return self.port(key[:-1])
        else:
            return self.port(key)

    def ctx_asgn(self, port: str, rhs: ExprBuilder):
        """Add an assignment to the current group context."""
        ctx_asgn(self.port(port), rhs)

    def __setitem__(self, key, value):
        ctx_asgn(self.port(key), value)

    def __setattr__(self, key, value):
        if key.startswith('_'):
            object.__setattr__(self, key, value)
        elif key.endswith('_'):
            self.ctx_asgn(key[:-1], value)
        else:
            self.ctx_asgn(key, value)


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

    def __init__(self, group: ast.Group, comp: ComponentBuilder):
        self.group = group
        self.comp = comp

    def asgn(self, lhs: ExprBuilder,
             rhs: ExprBuilder | CondExprBuilder | int,
             cond=None):
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
            assert width, f'could not infer width for literal {rhs}'
            rhs = const(width, rhs)

        wire = ast.Connect(
            ExprBuilder.unwrap(lhs),
            ExprBuilder.unwrap(rhs),
            ExprBuilder.unwrap(cond),
        )
        self.group.connections.append(wire)

    @property
    def done(self):
        """The `done` hole for the group."""
        return ExprBuilder(
            ast.HolePort(ast.CompVar(self.group.id.name), "done")
        )

    @done.setter
    def done(self, expr):
        """Build an assignment to `done` in the group."""
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
    group_builder = TLS.groups[-1]

    # Deal with `done` holes.
    expr = ExprBuilder.unwrap(expr)
    if isinstance(expr, ast.HolePort):
        assert expr.name == 'done', f"unknown hole {expr.name}"
        return 1

    # Otherwise, it's a `cell.port` lookup.
    assert isinstance(expr, ast.Atom)
    cell_name = expr.item.id.name
    port_name = expr.item.name

    # Look up the component for the referenced cell.
    cell_builder = group_builder.comp.index[cell_name]
    inst = cell_builder._cell.comp

    # Extract widths from stdlib components we know.
    if inst.id == 'std_reg':
        if port_name == 'in':
            return inst.args[0]
        if port_name == 'write_en':
            return 1

    # Give up.
    return None


def ctx_asgn(lhs: ExprBuilder, rhs: ExprBuilder | CondExprBuilder):
    """Add an assignment to the current group context."""
    assert TLS.groups, "assignment outside `with group`"
    group_builder = TLS.groups[-1]
    group_builder.asgn(lhs, rhs)
