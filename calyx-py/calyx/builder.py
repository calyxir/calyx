import threading
from calyx import py_ast as ast

# Thread-local storage to keep track of the current GroupBuilder we have
# entered as a context manager. This is weird magic!
TLS = threading.local()
TLS.groups = []


class ProgramBuilder:
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
    def __init__(self, name: str):
        self.component = ast.Component(
            name,
            inputs=[],
            outputs=[],
            structs=[],
            controls=ast.Empty(),
        )
        self.index = {}

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
        builder = GroupBuilder(group)
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

    Supports a magical subscript assignment operator to build a
    *conditional* assignment in the current group context. Use
    `cell.port[cond] = value` to build a Calyx assignment like
    `cell.port ? cond = value`.
    """

    def __init__(self, expr: ast.GuardExpr | ast.Port):
        self.expr = expr

    def __and__(self, other: 'ExprBuilder'):
        return ExprBuilder(ast.And(self.expr, other.expr))

    def __or__(self, other: 'ExprBuilder'):
        return ExprBuilder(ast.Or(self.expr, other.expr))

    def __invert__(self, other: 'ExprBuilder'):
        return ExprBuilder(ast.Not(self.expr))

    def ctx_cond_asgn(self, cond: 'ExprBuilder', rhs: 'ExprBuilder'):
        """Add a conditional assignment to the current group context."""
        assert TLS.groups, "conditional assignment outside `with group`"
        group_builder = TLS.groups[-1]
        group_builder.asgn(self, rhs, cond)

    def __setitem__(self, key, value):
        self.ctx_cond_asgn(key, value)

    @classmethod
    def unwrap(cls, obj):
        if isinstance(obj, cls):
            return obj.expr
        else:
            return obj


class CellBuilder:
    """Wraps a cell for convenient wire building.

    When we're in the context for a group builder (inside a `with
    group:` block), use `cell.port = expr` or `cell["port"] = expr` to
    add an assignment to the active group.

    Using "dot syntax" works only for port names that *do not* begin
    with a `_` (underscore fields are reserved for internal operations).
    """
    def __init__(self, cell: ast.Cell):
        self._cell = cell

    def port(self, name: str):
        """Build a port access expression."""
        return ExprBuilder(ast.CompPort(self._cell.id, name))

    def __getitem__(self, key):
        return self.port(key)

    def __getattr__(self, key):
        if key.startswith('_'):
            return object.__getattr__(self, key)
        else:
            return self.port(key)

    def ctx_asgn(self, port: str, rhs: ExprBuilder, cond=None):
        """Add an assignment to the current group context."""
        assert TLS.groups, "assignment outside `with group`"
        group_builder = TLS.groups[-1]
        group_builder.asgn(self.port(port), rhs, cond)

    def __setitem__(self, key, value):
        self.ctx_asgn(key, value)

    def __setattr__(self, key, value):
        if key.startswith('_'):
            object.__setattr__(self, key, value)
        else:
            self.ctx_asgn(key, value)


class GroupBuilder:
    def __init__(self, group: ast.Group):
        self.group = group

    def asgn(self, lhs, rhs, cond=None):
        """Add a connection to the group."""
        wire = ast.Connect(
            ExprBuilder.unwrap(rhs),
            ExprBuilder.unwrap(lhs),  # TODO Reverse.
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


# TODO Unfortunate.
def const(width: int, value: int):
    return ast.ConstantPort(width, value)
