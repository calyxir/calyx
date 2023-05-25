class ComponentBuilder:
    """Builds Calyx components definitions."""

    def __init__(
        self, prog: Builder, name: str, cells: Optional[List[ast.Cell]] = None
    ):
        """Contructs a new component in the current program. If `cells` is
        provided, the component will be initialized with those cells."""
        cells = cells if cells else list()
        self.prog = prog
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

    @property
    def control(self) -> ControlBuilder:
        """Access the component's control program."""
        return ControlBuilder(self.component.controls)

    @control.setter
    def control(self, builder: Union[ast.Control, ControlBuilder]):
        if isinstance(builder, ControlBuilder):
            self.component.controls = builder.stmt
        else:
            self.component.controls = builder

    def get_cell(self, name: str) -> CellBuilder:
        """Retrieve a cell builder by name."""
        out = self.index.get(name)
        if out and isinstance(out, CellBuilder):
            return out
        else:
            raise Exception(
                f"Cell `{name}' not found in component {self.component.name}.\n"
                f"Known cells: {list(map(lambda c: c.id.name, self.component.cells))}"
            )

    def get_group(self, name: str) -> GroupBuilder:
        """Retrieve a group builder by name."""
        out = self.index.get(name)
        if out and isinstance(out, GroupBuilder):
            return out
        else:
            raise Exception(
                f"Group `{name}' not found in component {self.component.name}"
            )

    def group(self, name: str, static_delay: Optional[int] = None) -> GroupBuilder:
        """Create a new group with the given name and (optional) static delay."""
        group = ast.Group(ast.CompVar(name), connections=[], static_delay=static_delay)
        assert group not in self.component.wires, f"group '{name}' already exists"

        self.component.wires.append(group)
        builder = GroupBuilder(group, self)
        self.index[name] = builder
        return builder

    def comb_group(self, name: str) -> GroupBuilder:
        """Create a new combinational group with the given name."""
        group = ast.CombGroup(ast.CompVar(name), connections=[])
        assert group not in self.component.wires, f"comb group '{name}' already exists"

        self.component.wires.append(group)
        builder = GroupBuilder(group, self)
        self.index[name] = builder
        return builder

    def cell(
        self,
        name: str,
        comp: Union[ast.CompInst, ComponentBuilder],
        is_external=False,
        is_ref=False,
    ) -> CellBuilder:
        """Declare a cell in the component. Returns a cell builder."""
        # If we get a (non-primitive) component builder, instantiate it
        # with no parameters.
        if isinstance(comp, ComponentBuilder):
            comp = ast.CompInst(comp.component.name, [])

        cell = ast.Cell(ast.CompVar(name), comp, is_external, is_ref)
        assert cell not in self.component.cells, f"cell '{name}' already exists"

        self.component.cells.append(cell)
        builder = CellBuilder(cell)
        self.index[name] = builder
        return builder

    def comp_instance(
        self,
        cell_name: str,
        comp_name: str | ComponentBuilder,
        check_undeclared=True,
    ) -> CellBuilder:
        """Create a cell for a Calyx sub-component.

        This is primarily for when the instantiated component has not yet been
        defined. When the component has been defined, use `cell` instead with
        the `ComponentBuilder` object.
        """
        if isinstance(comp_name, str):
            assert not check_undeclared or (
                comp_name in self.prog._index
                or comp_name in self.prog.program.components
            ), (
                f"Declaration of component '{comp_name}' not found in program. If this "
                "is expected, set `check_undeclared=False`."
            )

        if isinstance(comp_name, ComponentBuilder):
            comp_name = comp_name.component.name

        return self.cell(cell_name, ast.CompInst(comp_name, []))

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
        gl = obj.group_like
        assert gl, (
            "GroupBuilder represents continuous assignments and"
            " cannot be used as a control statement"
        )
        assert not isinstance(
            gl, ast.CombGroup
        ), "Cannot use combinational group as control statement"
        return ast.Enable(gl.id.name)
    elif isinstance(obj, list):
        return ast.SeqComp([as_control(o) for o in obj])
    elif isinstance(obj, set):
        raise TypeError(
            "Python sets are not supported in control programs. For a parallel"
            " composition use `Builder.par` instead."
        )
    elif obj is None:
        return ast.Empty()
    else:
        assert False, f"unsupported control type {type(obj)}"


def while_(port: ExprBuilder, cond: Optional[GroupBuilder], body) -> ast.While:
    """Build a `while` control statement."""
    if cond:
        assert isinstance(
            cond.group_like, ast.CombGroup
        ), "while condition must be a combinational group"
        cg = cond.group_like.id
    else:
        cg = None
    return ast.While(port.expr, cg, as_control(body))


def if_(
    port: ExprBuilder,
    cond: Optional[GroupBuilder],
    body,
    else_body=None,
) -> ast.If:
    """Build an `if` control statement."""
    else_body = ast.Empty() if else_body is None else else_body

    if cond:
        assert isinstance(
            cond.group_like, ast.CombGroup
        ), "if condition must be a combinational group"
        cg = cond.group_like.id
    else:
        cg = None
    return ast.If(port.expr, cg, as_control(body), as_control(else_body))


def invoke(cell: CellBuilder, **kwargs) -> ast.Invoke:
    """Build an `invoke` control statement.

    The keyword arguments should have the form `in_*`, `out_*`, or `ref_*`, where
    `*` is the name of an input port, output port, or ref cell on the invoked cell.
    """
    return ast.Invoke(
        cell._cell.id,
        [
            (k[3:], ExprBuilder.unwrap(v))
            for (k, v) in kwargs.items()
            if k.startswith("in_")
        ],
        [
            (k[4:], ExprBuilder.unwrap(v))
            for (k, v) in kwargs.items()
            if k.startswith("out_")
        ],
        [
            (k[4:], CellBuilder.unwrap_id(v))
            for (k, v) in kwargs.items()
            if k.startswith("ref_")
        ],
    )


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

    def __and__(self, other: ExprBuilder):
        """Construct an "and" logical expression with &."""
        return ExprBuilder(ast.And(self.expr, other.expr))

    def __or__(self, other: ExprBuilder):
        """Construct an "or" logical expression with |."""
        return ExprBuilder(ast.Or(self.expr, other.expr))

    def __invert__(self):
        """Construct a "not" logical expression with ~."""
        return ExprBuilder(ast.Not(self.expr))

    def __matmul__(self, rhs: ExprBuilder):
        """Construct a conditional expression with @.

        This produces a `CondExprBuilder`, which wraps a value
        expression and a logical condition expression. Assigning this
        into any port or hole *makes that assignment conditional*. It is
        not possible to compose larger expressions out of
        `CondExprBuilder` values; they are only useful for assignment.
        """
        return CondExprBuilder(self, rhs)

    def __eq__(self, other: ExprBuilder):
        """Construct an equality comparison with ==."""
        return ExprBuilder(ast.Eq(self.expr, other.expr))

    def __ne__(self, other: ExprBuilder):
        """Construct an inequality comparison with ==."""
        return ExprBuilder(ast.Neq(self.expr, other.expr))

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

    def port(self, name: str) -> ExprBuilder:
        """Build a port access expression."""
        return ExprBuilder(ast.Atom(ast.CompPort(self._cell.id, name)))

    @classmethod
    def unwrap_id(cls, obj):
        if isinstance(obj, cls):
            return obj._cell.id
        else:
            return obj


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

    def __init__(
        self,
        group_like: Optional[Union[ast.Group, ast.CombGroup]],
        comp: ComponentBuilder,
    ):
        self.group_like = group_like
        self.comp = comp

    def as_enable(self) -> Optional[ast.Enable]:
        if isinstance(self.group_like, ast.Group):
            return ast.Enable(self.group_like.id.name)
        else:
            return None

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

        assert isinstance(rhs, (ExprBuilder, ast.Port)), (
            "assignment must use literal int, conditional, or expression, "
            f"not {type(rhs)}"
        )

        wire = ast.Connect(
            ExprBuilder.unwrap(lhs),
            ExprBuilder.unwrap(rhs),
            ExprBuilder.unwrap(cond),
        )
        if self.group_like:
            self.group_like.connections.append(wire)
        else:
            self.comp.component.wires.append(wire)

    @property
    def done(self):
        """The `done` hole for the group."""
        assert self.group_like, (
            "GroupLikeBuilder represents continuous assignments"
            " and does not have a done hole"
        )
        assert not isinstance(
            self.group_like, ast.CombGroup
        ), "done hole not available for comb group"

        return ExprBuilder(ast.HolePort(ast.CompVar(self.group_like.id.name), "done"))

    @done.setter
    def done(self, expr):
        """Build an assignment to `done` in the group."""
        if not self.group_like:
            raise Exception(
                "GroupBuilder represents continuous assignments and does not have a done hole"
            )
        self.asgn(self.done, expr)

    def __enter__(self):
        TLS.groups.append(self)
        return self

    def __exit__(self, exc, value, tb):
        TLS.groups.pop()


def ctx_asgn(lhs: ExprBuilder, rhs: Union[ExprBuilder, CondExprBuilder]):
    """Add an assignment to the current group context."""
    assert TLS.groups, "assignment outside `with group`"
    group_builder: GroupBuilder = TLS.groups[-1]
    group_builder.asgn(lhs, rhs)


def par(*args) -> ast.ParComp:
    """Build a parallel composition of control expressions.

    Each argument will become its own parallel arm in the resulting composition.
    So `par([a,b])` becomes `par {seq {a; b;}}` while `par(a, b)` becomes `par {a; b;}`.
    """
    return ast.ParComp([as_control(x) for x in args])
