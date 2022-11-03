from calyx import py_ast as ast


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


class CellBuilder:
    def __init__(self, cell: ast.Cell):
        self.cell = cell

    def port(self, name: str):
        """Build a port access expression."""
        return ExprBuilder(ast.CompPort(self.cell.id, name))


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


class ExprBuilder:
    """Wraps an assignment expression.

    This wrapper provides convenient ways to build logical expressions
    for guards. Use the Python operators &, |, and ~ to build and, or,
    and not expressions in Calyx.
    """

    def __init__(self, expr: ast.GuardExpr | ast.Port):
        self.expr = expr

    def __and__(self, other: 'ExprBuilder'):
        return ExprBuilder(ast.And(self.expr, other.expr))

    def __or__(self, other: 'ExprBuilder'):
        return ExprBuilder(ast.Or(self.expr, other.expr))

    def __invert__(self, other: 'ExprBuilder'):
        return ExprBuilder(ast.Not(self.expr))

    @classmethod
    def unwrap(cls, obj):
        if isinstance(obj, cls):
            return obj.expr
        else:
            return obj


# TODO Unfortunate.
def const(width: int, value: int):
    return ast.ConstantPort(width, value)
