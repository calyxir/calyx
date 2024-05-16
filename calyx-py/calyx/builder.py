from __future__ import annotations

import threading
from typing import Dict, Union, Optional, List
from dataclasses import dataclass
from . import py_ast as ast

# Thread-local storage to keep track of the current GroupBuilder we have
# entered as a context manager. This is weird magic!
TLS = threading.local()
TLS.groups = []


class NotFoundError(Exception):
    """Raised when a component or group is not found."""


class WidthInferenceError(Exception):
    """Raised when we cannot infer the width of an expression."""


class MalformedGroupError(Exception):
    """Raised when a group is malformed."""


class Builder:
    """The entry-point builder for top-level Calyx programs."""

    def __init__(self):
        self.program = ast.Program(
            imports=[],
            components=[],
        )
        self.imported = set()
        self.import_("primitives/core.futil")
        self._index: Dict[str, ComponentBuilder] = {}

    def component(self, name: str, latency=None) -> ComponentBuilder:
        """Create a new component builder."""
        comp_builder = ComponentBuilder(self, name, latency)
        self.program.components.append(comp_builder.component)
        self._index[name] = comp_builder
        return comp_builder

    def get_component(self, name: str) -> ComponentBuilder:
        """Retrieve a component builder by name."""
        comp_builder = self._index.get(name)
        if comp_builder is None:
            raise NotFoundError(f"Component `{name}' not found in program.")
        else:
            return comp_builder

    def import_(self, filename: str):
        """Add an `import` statement to the program."""
        if filename not in self.imported:
            self.imported.add(filename)
            self.program.imports.append(ast.Import(filename))


class ComponentBuilder:
    """Builds Calyx components definitions."""

    next_gen_idx = 0

    def __init__(
        self,
        prog: Builder,
        name: str,
        latency: Optional[int] = None,
    ):
        """Contructs a new component in the current program."""
        self.prog = prog
        self.component: ast.Component = ast.Component(
            name,
            attributes=[],
            inputs=[],
            outputs=[],
            structs=list(),
            controls=ast.Empty(),
            latency=latency,
        )
        self.index: Dict[str, Union[GroupBuilder, CellBuilder]] = {}
        self.continuous = GroupBuilder(None, self)
        self.next_gen_idx = 0

    def generate_name(self, prefix: str) -> str:
        """Generate a unique name with the given prefix."""
        while True:
            self.next_gen_idx += 1
            name = f"{prefix}_{self.next_gen_idx}"
            if name not in self.index:
                return name

    def input(self, name: str, size: int) -> ExprBuilder:
        """Declare an input port on the component.

        Returns an expression builder for the port.
        """
        self.component.inputs.append(ast.PortDef(ast.CompVar(name), size))
        return self.this()[name]

    def output(self, name: str, size: int) -> ExprBuilder:
        """Declare an output port on the component.

        Returns an expression builder for the port.
        """
        self.component.outputs.append(ast.PortDef(ast.CompVar(name), size))
        return self.this()[name]

    def attribute(self, name: str, value: int) -> None:
        """Declare an attribute on the component."""
        self.component.attributes.append(ast.CompAttribute(name, value))

    def this(self) -> ThisBuilder:
        """Get a handle to the component's `this` cell.

        This is used to access the component's input/output ports with the
        standard `this.port` syntax.
        """
        return ThisBuilder()

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

    def port_width(self, port: ExprBuilder) -> int:
        """Get the width of an expression, which may be a port of this component."""
        name = ExprBuilder.unwrap(port).item.id.name
        for input in self.component.inputs:
            if input.id.name == name:
                return input.width
        for output in self.component.outputs:
            if output.id.name == name:
                return output.width
        # Give up.
        return None

    def get_cell(self, name: str) -> CellBuilder:
        """Retrieve a cell builder by name."""
        out = self.index.get(name)
        if out and isinstance(out, CellBuilder):
            return out
        else:
            raise NotFoundError(
                f"Cell `{name}' not found in component {self.component.name}.\n"
                f"Known cells: {list(map(lambda c: c.id.name, self.component.cells))}"
            )

    def try_get_cell(self, name: str) -> CellBuilder:
        """Tries to get a cell builder by name. If cannot find it, return None"""
        out = self.index.get(name)
        if out and isinstance(out, CellBuilder):
            return out
        else:
            return None

    def get_group(self, name: str) -> GroupBuilder:
        """Retrieve a group builder by name."""
        out = self.index.get(name)
        if out and isinstance(out, GroupBuilder):
            return out
        else:
            raise NotFoundError(
                f"Group `{name}' not found in component {self.component.name}"
            )

    def try_get_group(self, name: str) -> GroupBuilder:
        """Tries to get a group builder by name. If cannot find it, return None"""
        out = self.index.get(name)
        if out and isinstance(out, GroupBuilder):
            return out
        else:
            return None

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
        assert group not in self.component.wires, f"group '{name}' already exists"

        self.component.wires.append(group)
        builder = GroupBuilder(group, self)
        self.index[name] = builder
        return builder

    def static_group(self, name: str, latency: int) -> GroupBuilder:
        """Create a new combinational group with the given name."""
        group = ast.StaticGroup(ast.CompVar(name), connections=[], latency=latency)
        assert group not in self.component.wires, f"group '{name}' already exists"

        self.component.wires.append(group)
        builder = GroupBuilder(group, self)
        self.index[name] = builder
        return builder

    def cell(
        self,
        name: str,
        comp: Union[ast.CompInst, ComponentBuilder],
        is_external: bool = False,
        is_ref: bool = False,
    ) -> CellBuilder:
        """Declare a cell in the component. Return a cell builder."""
        # If given a (non-primitive) component builder, instantiate it
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

    def reg(self, size: int, name: str = None, is_ref: bool = False) -> CellBuilder:
        """Generate a StdReg cell."""
        assert isinstance(size, int), f"size {size} is not an int"
        if name:
            assert isinstance(name, str), f"name {name} is not a string"
        if is_ref and not name:
            raise ValueError(
                "A register that will be passed by reference must have a name."
            )
        name = name or self.generate_name("reg")
        return self.cell(name, ast.Stdlib.register(size), False, is_ref)

    def wire(self, name: str, size: int, is_ref: bool = False) -> CellBuilder:
        """Generate a StdWire cell."""
        return self.cell(name, ast.Stdlib.wire(size), False, is_ref)

    def slice(
        self,
        name: str,
        in_width: int,
        out_width: int,
        is_ref: bool = False,
    ) -> CellBuilder:
        """Generate a StdSlice cell."""
        return self.cell(name, ast.Stdlib.slice(in_width, out_width), False, is_ref)

    def const(self, name: str, width: int, value: int) -> CellBuilder:
        """Generate a StdConstant cell."""
        return self.cell(name, ast.Stdlib.constant(width, value))

    def comb_mem_d1(
        self,
        name: str,
        bitwidth: int,
        len: int,
        idx_size: int,
        is_external: bool = False,
        is_ref: bool = False,
    ) -> CellBuilder:
        """Generate a StdMemD1 cell."""
        self.prog.import_("primitives/memories/comb.futil")
        return self.cell(
            name, ast.Stdlib.comb_mem_d1(bitwidth, len, idx_size), is_external, is_ref
        )

    def seq_mem_d1(
        self,
        name: str,
        bitwidth: int,
        len: int,
        idx_size: int,
        is_external: bool = False,
        is_ref: bool = False,
    ) -> CellBuilder:
        """Generate a SeqMemD1 cell."""
        self.prog.import_("primitives/memories/seq.futil")
        return self.cell(
            name, ast.Stdlib.seq_mem_d1(bitwidth, len, idx_size), is_external, is_ref
        )

    def binary(
        self,
        operation: str,
        size: int,
        name: Optional[str] = None,
        signed: bool = False,
    ) -> CellBuilder:
        """Generate a binary cell of the kind specified in `operation`."""
        self.prog.import_("primitives/binary_operators.futil")
        name = name or self.generate_name(operation)
        assert isinstance(name, str), f"name {name} is not a string"
        return self.cell(name, ast.Stdlib.op(operation, size, signed))

    def add(self, size: int, name: str = None, signed: bool = False) -> CellBuilder:
        """Generate a StdAdd cell."""
        return self.binary("add", size, name, signed)

    def sub(self, size: int, name: str = None, signed: bool = False) -> CellBuilder:
        """Generate a StdSub cell."""
        return self.binary("sub", size, name, signed)

    def div_pipe(
        self, size: int, name: str = None, signed: bool = False
    ) -> CellBuilder:
        """Generate a Div_Pipe cell."""
        return self.binary("div_pipe", size, name, signed)

    def mult_pipe(
        self, size: int, name: str = None, signed: bool = False
    ) -> CellBuilder:
        """Generate a Mult_Pipe cell."""
        return self.binary("mult_pipe", size, name, signed)

    def gt(self, size: int, name: str = None, signed: bool = False) -> CellBuilder:
        """Generate a StdGt cell."""
        return self.binary("gt", size, name, signed)

    def lt(self, size: int, name: str = None, signed: bool = False) -> CellBuilder:
        """Generate a StdLt cell."""
        return self.binary("lt", size, name, signed)

    def eq(self, size: int, name: str = None, signed: bool = False) -> CellBuilder:
        """Generate a StdEq cell."""
        return self.binary("eq", size, name, signed)

    def neq(self, size: int, name: str = None, signed: bool = False) -> CellBuilder:
        """Generate a StdNeq cell."""
        return self.binary("neq", size, name, signed)

    def ge(self, size: int, name: str = None, signed: bool = False) -> CellBuilder:
        """Generate a StdGe cell."""
        return self.binary("ge", size, name, signed)

    def le(self, size: int, name: str = None, signed: bool = False) -> CellBuilder:
        """Generate a StdLe cell."""
        return self.binary("le", size, name, signed)

    def rsh(self, size: int, name: str = None, signed: bool = False) -> CellBuilder:
        """Generate a StdRsh cell."""
        return self.binary("rsh", size, name, signed)

    def lsh(self, size: int, name: str = None, signed: bool = False) -> CellBuilder:
        """Generate a StdLsh cell."""
        return self.binary("lsh", size, name, signed)

    def logic(self, operation, size: int, name: str = None) -> CellBuilder:
        """Generate a logical operator cell, of the flavor specified in `operation`."""
        name = name or self.generate_name(operation)
        assert isinstance(name, str)
        return self.cell(name, ast.Stdlib.op(operation, size, False))

    def and_(self, size: int, name: str = None) -> CellBuilder:
        """Generate a StdAnd cell."""
        name = name or self.generate_name("and")
        return self.logic("and", size, name)

    def not_(self, size: int, name: str = None) -> CellBuilder:
        """Generate a StdNot cell."""
        name = name or self.generate_name("not")
        return self.logic("not", size, name)

    def pipelined_mult(self, name: str) -> CellBuilder:
        """Generate a pipelined multiplier."""
        self.prog.import_("primitives/pipelined.futil")
        return self.cell(name, ast.Stdlib.pipelined_mult())

    def pipelined_fp_smult(
        self, name: str, width, int_width, frac_width
    ) -> CellBuilder:
        """Generate a pipelined fixed point signed multiplier."""
        self.prog.import_("primitives/pipelined.futil")
        return self.cell(
            name, ast.Stdlib.pipelined_fp_smult(width, int_width, frac_width)
        )

    def fp_op(
        self,
        cell_name: str,
        op_name,
        width: int,
        int_width: int,
        frac_width: int,
    ) -> CellBuilder:
        """Generate an UNSIGNED fixed point op."""
        self.prog.import_("primitives/binary_operators.futil")
        return self.cell(
            cell_name,
            ast.Stdlib.fixed_point_op(op_name, width, int_width, frac_width, False),
        )

    def fp_sop(
        self,
        cell_name: str,
        op_name,
        width: int,
        int_width: int,
        frac_width: int,
    ) -> CellBuilder:
        """Generate a SIGNED fixed point op."""
        self.prog.import_("primitives/binary_operators.futil")
        return self.cell(
            cell_name,
            ast.Stdlib.fixed_point_op(op_name, width, int_width, frac_width, True),
        )

    def unary_use(self, input, cell, groupname=None):
        """Accepts a cell that performs some computation on value `input`.
        Creates a combinational group that wires up the cell with this port.
        Returns the cell and the combintational group.

        comb group `groupname` {
            `cell.name`.in = `input`;
        }

        Returns handles to the cell and the combinational group.
        """
        groupname = groupname or f"{cell.name}_group"
        with self.comb_group(groupname) as comb_group:
            cell.in_ = input
        return CellAndGroup(cell, comb_group)

    def binary_use(self, left, right, cell, groupname=None):
        """Accepts a cell that performs some computation on values `left` and `right`.
        Creates a combinational group that wires up the cell with these ports.
        Returns the cell and the combintational group.

        comb group `groupname` {
            `cell.name`.left = `left`;
            `cell.name`.right = `right`;
        }

        Returns handles to the cell and the combinational group.
        """
        groupname = groupname or f"{cell.name}_group"
        with self.comb_group(groupname) as comb_group:
            cell.left = left
            cell.right = right
        return CellAndGroup(cell, comb_group)

    def binary_use_names(self, cellname, leftname, rightname, groupname=None):
        """Accepts the name of a cell that performs some computation on two values.
        Accepts the names of cells that contain those two values.
        Creates a group that wires up the cell with those values.
        Returns the group created.

        group `groupname` {
            `cellname`.left = `leftname`.out;
            `cellname`.right = `rightname`.out;
            `groupname`.go = 1;
            `groupname`.done = `cellname`.done;
        }
        """
        cell = self.get_cell(cellname)
        groupname = groupname or f"{cellname}_group"
        with self.group(groupname) as group:
            cell.left = self.get_cell(leftname).out
            cell.right = self.get_cell(rightname).out
            cell.go = HI
            group.done = cell.done
        return group

    def try_infer_width(self, width, left, right):
        """If `width` is None, try to infer it from `left` or `right`.
        If that fails, raise an error.
        """
        width = width or self.infer_width(left) or self.infer_width(right)
        if not width:
            raise WidthInferenceError(
                "Cannot infer widths from `left` or `right`. "
                "Consider providing width as an argument."
            )
        return width

    def eq_use(self, left, right, signed=False, cellname=None, width=None):
        """Inserts wiring into `self` to check if `left` == `right`."""
        width = self.try_infer_width(width, left, right)
        return self.binary_use(left, right, self.eq(width, cellname, signed))

    def neq_use(self, left, right, signed=False, cellname=None, width=None):
        """Inserts wiring into `self` to check if `left` != `right`."""
        width = self.try_infer_width(width, left, right)
        return self.binary_use(left, right, self.neq(width, cellname, signed))

    def lt_use(self, left, right, signed=False, cellname=None, width=None):
        """Inserts wiring into `self` to check if `left` < `right`."""
        width = self.try_infer_width(width, left, right)
        return self.binary_use(left, right, self.lt(width, cellname, signed))

    def le_use(self, left, right, signed=False, cellname=None, width=None):
        """Inserts wiring into `self` to check if `left` <= `right`."""
        width = self.try_infer_width(width, left, right)
        return self.binary_use(left, right, self.le(width, cellname, signed))

    def ge_use(self, left, right, signed=False, cellname=None, width=None):
        """Inserts wiring into `self` to check if `left` >= `right`."""
        width = self.try_infer_width(width, left, right)
        return self.binary_use(left, right, self.ge(width, cellname, signed))

    def gt_use(self, left, right, signed=False, cellname=None, width=None):
        """Inserts wiring into `self` to check if `left` > `right`."""
        width = self.try_infer_width(width, left, right)
        return self.binary_use(left, right, self.gt(width, cellname, signed))

    def add_use(self, left, right, signed=False, cellname=None, width=None):
        """Inserts wiring into `self` to compute `left` + `right`."""
        width = self.try_infer_width(width, left, right)
        return self.binary_use(left, right, self.add(width, cellname, signed))

    def sub_use(self, left, right, signed=False, cellname=None, width=None):
        """Inserts wiring into `self` to compute `left` - `right`."""
        width = self.try_infer_width(width, left, right)
        return self.binary_use(left, right, self.sub(width, cellname, signed))

    def not_use(self, input, cellname=None, width=None):
        """Inserts wiring into `self` to compute `not input`."""
        width = self.try_infer_width(width, input, input)
        return self.unary_use(input, self.not_(width, cellname))

    def bitwise_flip_reg(self, reg, cellname=None):
        """Inserts wiring into `self` to bitwise-flip the contents of `reg`
        and put the result back into `reg`.
        """
        cellname = cellname or f"{reg.name}_not"
        width = reg.infer_width_reg()
        not_cell = self.not_(width, cellname)
        with self.group(f"{cellname}_group") as not_group:
            not_cell.in_ = reg.out
            reg.write_en = 1
            reg.in_ = not_cell.out
            not_group.done = reg.done
        return not_group

    def incr(self, reg, val=1, signed=False, cellname=None, static=False):
        """Inserts wiring into `self` to perform `reg := reg + val`."""
        cellname = cellname or f"{reg.name}_incr"
        width = reg.infer_width_reg()
        add_cell = self.add(width, cellname, signed)
        group = (
            self.static_group(f"{cellname}_group", 1)
            if static
            else self.group(f"{cellname}_group")
        )
        with group as incr_group:
            add_cell.left = reg.out
            add_cell.right = const(width, val)
            reg.write_en = 1
            reg.in_ = add_cell.out
            if not static:
                incr_group.done = reg.done
        return incr_group

    def decr(self, reg, val=1, signed=False, cellname=None):
        """Inserts wiring into `self` to perform `reg := reg - val`."""
        cellname = cellname or f"{reg.name}_decr"
        width = reg.infer_width_reg()
        sub_cell = self.sub(width, cellname, signed)
        with self.group(f"{cellname}_group") as decr_group:
            sub_cell.left = reg.out
            sub_cell.right = const(width, val)
            reg.write_en = 1
            reg.in_ = sub_cell.out
            decr_group.done = reg.done
        return decr_group

    def lsh_use(self, input, ans, val=1):
        """Inserts wiring into `self` to perform `ans := input << val`."""
        width = ans.infer_width_reg()
        cell = self.lsh(width)
        with self.group(f"{cell.name}_group") as lsh_group:
            cell.left = input
            cell.right = const(width, val)
            ans.write_en = 1
            ans.in_ = cell.out
            lsh_group.done = ans.done
        return lsh_group

    def rsh_use(self, input, ans, val=1):
        """Inserts wiring into `self` to perform `ans := input >> val`."""
        width = ans.infer_width_reg()
        cell = self.rsh(width)
        with self.group(f"{cell.name}_group") as rsh_group:
            cell.left = input
            cell.right = const(width, val)
            ans.write_en = 1
            ans.in_ = cell.out
            rsh_group.done = ans.done
        return rsh_group

    def reg_store(self, reg, val, groupname=None):
        """Inserts wiring into `self` to perform `reg := val`."""
        groupname = groupname or f"{reg.name}_store_to_reg"
        with self.group(groupname) as reg_grp:
            reg.in_ = val
            reg.write_en = 1
            reg_grp.done = reg.done
        return reg_grp

    def mem_load_comb_mem_d1(self, mem, i, reg, groupname=None):
        """Inserts wiring into `self` to perform `reg := mem[i]`,
        where `mem` is a comb_mem_d1 memory.
        """
        assert mem.is_comb_mem_d1()
        groupname = groupname or f"{mem.name()}_load_to_reg"
        with self.group(groupname) as load_grp:
            mem.addr0 = i
            reg.write_en = 1
            reg.in_ = mem.read_data
            load_grp.done = reg.done
        return load_grp

    def mem_store_comb_mem_d1(self, mem, i, val, groupname=None):
        """Inserts wiring into `self` to perform `mem[i] := val`,
        where `mem` is a comb_mem_d1 memory."""
        assert mem.is_comb_mem_d1()
        groupname = groupname or f"store_into_{mem.name()}"
        with self.group(groupname) as store_grp:
            mem.addr0 = i
            mem.write_en = 1
            mem.write_data = val
            store_grp.done = mem.done
        return store_grp

    def mem_read_seq_d1(self, mem, i, groupname=None):
        """Inserts wiring into `self` to latch `mem[i]` as the output of `mem`,
        where `mem` is a seq_d1 memory.
        Note that this does not write the value anywhere.
        """
        assert mem.is_seq_mem_d1()
        groupname = groupname or f"read_from_{mem.name()}"
        with self.group(groupname) as read_grp:
            mem.addr0 = i
            mem.content_en = 1
            read_grp.done = mem.done
        return read_grp

    def mem_write_seq_d1_to_reg(self, mem, reg, groupname=None):
        """Inserts wiring into `self` to perform reg := <mem_latched_value>,
        where `mem` is a seq_d1 memory that already has some latched value.
        """
        assert mem.is_seq_mem_d1()
        groupname = groupname or f"{mem.name()}_write_to_reg"
        with self.group(groupname) as write_grp:
            reg.write_en = 1
            reg.in_ = mem.read_data
            write_grp.done = reg.done
        return write_grp

    def mem_store_seq_d1(self, mem, i, val, groupname=None):
        """Inserts wiring into `self` to perform `mem[i] := val`,
        where `mem` is a seq_d1 memory.
        """
        assert mem.is_seq_mem_d1()
        groupname = groupname or f"{mem.name()}_store"
        with self.group(groupname) as store_grp:
            mem.addr0 = i
            mem.write_en = 1
            mem.write_data = val
            mem.content_en = 1
            store_grp.done = mem.done
        return store_grp

    def mem_load_to_mem(self, mem, i, ans, j, groupname=None):
        """Inserts wiring into `self` to perform `ans[j] := mem[i]`,
        where `mem` and `ans` are both comb_mem_d1 memories.
        """
        assert mem.is_comb_mem_d1() and ans.is_comb_mem_d1()
        groupname = groupname or f"{mem.name()}_load_to_mem"
        with self.group(groupname) as load_grp:
            mem.addr0 = i
            ans.write_en = 1
            ans.addr0 = j
            ans.write_data = mem.read_data
            load_grp.done = ans.done
        return load_grp

    def op_store_in_reg(
        self,
        op_cell,
        left,
        right,
        cellname,
        width,
        ans_reg=None,
    ):
        """Inserts wiring into `self` to perform `reg := left op right`,
        where `op_cell`, a Cell that performs some `op`, is provided.
        """
        ans_reg = ans_reg or self.reg(width, f"reg_{cellname}")
        with self.group(f"{cellname}_group") as op_group:
            op_cell.left = left
            op_cell.right = right
            ans_reg.write_en = 1
            ans_reg.in_ = op_cell.out
            op_group.done = ans_reg.done
        return op_group, ans_reg

    def add_store_in_reg(
        self,
        left,
        right,
        ans_reg=None,
        cellname=None,
        width=None,
        signed=False,
    ):
        """Inserts wiring into `self` to perform `reg := left + right`."""
        width = width or self.try_infer_width(width, left, right)
        cell = self.add(width, cellname, signed)
        return self.op_store_in_reg(cell, left, right, cell.name, width, ans_reg)

    def sub_store_in_reg(
        self,
        left,
        right,
        ans_reg=None,
        cellname=None,
        width=None,
        signed=False,
    ):
        """Inserts wiring into `self` to perform `reg := left - right`."""
        width = width or self.try_infer_width(width, left, right)
        cell = self.sub(width, cellname, signed)
        return self.op_store_in_reg(cell, left, right, cell.name, width, ans_reg)

    def eq_store_in_reg(
        self,
        left,
        right,
        ans_reg=None,
        cellname=None,
        width=None,
        signed=False,
    ):
        """Inserts wiring into `self` to perform `reg := left == right`."""
        width = width or self.try_infer_width(width, left, right)
        cell = self.eq(width, cellname, signed)
        return self.op_store_in_reg(cell, left, right, cell.name, 1, ans_reg)

    def neq_store_in_reg(
        self,
        left,
        right,
        ans_reg=None,
        cellname=None,
        width=None,
        signed=False,
    ):
        """Inserts wiring into `self` to perform `reg := left != right`."""
        width = width or self.try_infer_width(width, left, right)
        cell = self.neq(width, cellname, signed)
        return self.op_store_in_reg(cell, left, right, cell.name, 1, ans_reg)

    def infer_width(self, expr) -> int:
        """Infer the width of an expression."""
        if isinstance(expr, int):  # We can't infer the width of an integer.
            return None
        if self.port_width(expr):  # It's an in/out port of this component!
            return self.port_width(expr)
        expr = ExprBuilder.unwrap(expr)  # We unwrap the expr.
        if isinstance(expr, ast.Atom):  # Inferring width of Atom.
            if isinstance(expr.item, ast.ThisPort):  # Atom is a ThisPort.
                # If we can infer it from this, great, otherwise give up.
                return self.port_width(expr)
            # Not a ThisPort, but maybe some `cell.port`?
            cell_name = expr.item.id.name
            port_name = expr.item.name
            cell_builder = self.index[cell_name]
            if not isinstance(cell_builder, CellBuilder):
                return None  # Something is wrong, we should have a CellBuilder
            # Okay, we really have a CellBuilder.
            # Let's try to infer the width of the port.
            # If this fails, give up.
            return cell_builder.infer_width(port_name)


@dataclass(frozen=True)
class CellAndGroup:
    """Just a cell and a group, for when it is convenient to
    pass them around together.

    Typically the group will be a combinational group, and `if_with` and
    `while_with` will require that a CellAndGroup be passed in, not a
    cell and a group separately.
    """

    cell: CellBuilder
    group: GroupBuilder


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
    if isinstance(obj, str):
        return ast.Enable(obj)
    if isinstance(obj, ast.Group):
        return ast.Enable(obj.id.name)
    if isinstance(obj, GroupBuilder):
        gl = obj.group_like
        assert gl, (
            "GroupBuilder represents continuous assignments and"
            " cannot be used as a control statement"
        )
        assert not isinstance(
            gl, ast.CombGroup
        ), "Cannot use combinational group as control statement"
        return ast.Enable(gl.id.name)
    if isinstance(obj, list):
        return ast.SeqComp([as_control(o) for o in obj])
    if isinstance(obj, set):
        raise TypeError(
            "Python sets are not supported in control programs. For a parallel"
            " composition use `Builder.par` instead."
        )
    if obj is None or obj == ast.Empty:
        return ast.Empty()
    else:
        assert False, f"unsupported control type {type(obj)}"


def while_(port: ExprBuilder, body) -> ast.While:
    """Build a `while` control statement.

    To build a `while` statement with a combinational group, use `while_with`.
    """
    return ast.While(port.expr, None, as_control(body))


def static_repeat(num_repeats: int, body) -> ast.StaticRepeat:
    """Build a `static repeat` control statement."""
    return ast.StaticRepeat(num_repeats, as_control(body))


def if_(
    port: ExprBuilder,
    body,
    else_body=None,
) -> ast.If:
    """Build an `if` control statement.

    To build an `if` statement with a combinational group, use `if_with`.
    """
    else_body = else_body or ast.Empty()
    return ast.If(port.expr, None, as_control(body), as_control(else_body))


def static_if(
    port: ExprBuilder,
    body,
    else_body=None,
) -> ast.If:
    """Build a `static if` control statement."""
    else_body = else_body or ast.Empty()
    return ast.StaticIf(port.expr, as_control(body), as_control(else_body))


def if_with(port_comb: CellAndGroup, body, else_body=None) -> ast.If:
    """Build an if statement, where the cell and the combinational group
    are provided together.
    """
    port = port_comb.cell.out
    cond = port_comb.group
    else_body = else_body or ast.Empty()

    assert isinstance(
        cond.group_like, ast.CombGroup
    ), "if condition must be a combinational group"
    return ast.If(
        port.expr, cond.group_like.id, as_control(body), as_control(else_body)
    )


def while_with(port_comb: CellAndGroup, body) -> ast.While:
    """Build a while statement, where the cell and the combinational
    group are provided together.
    """

    port = port_comb.cell.out
    cond = port_comb.group
    assert isinstance(
        cond.group_like, ast.CombGroup
    ), "while condition must be a combinational group"
    return ast.While(port.expr, cond.group_like.id, as_control(body))


def invoke(cell: CellBuilder, **kwargs) -> ast.Invoke:
    """Build an `invoke` control statement.

    The keyword arguments should have the form `in_*`, `out_*`, or `ref_*`, where
    `*` is the name of an input port, output port, or ref cell on the invoked cell.
    """
    return ast.Invoke(
        cell._cell.id,
        [
            (
                k[3:],
                (
                    (
                        const(cell.infer_width(k[3:]), v).expr
                        if isinstance(v, int)
                        else ExprBuilder.unwrap(v)
                    )
                ),
            )
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


def static_invoke(cell: CellBuilder, **kwargs) -> ast.Invoke:
    """Build a `static invoke` control statement.

    The keyword arguments should have the form `in_*`, `out_*`, or `ref_*`, where
    `*` is the name of an input port, output port, or ref cell on the invoked cell.
    """
    return ast.StaticInvoke(
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

    @property
    def name(self):
        """Get the name of the expression."""
        return self.expr.name

    @classmethod
    def unwrap(cls, obj):
        """Unwrap an expression builder, or return the object if it is not one."""
        if isinstance(obj, cls):
            return obj.expr
        return obj


@dataclass
class CondExprBuilder:
    """Wraps a conditional expression."""

    cond: ExprBuilder
    value: ExprBuilder


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

    def is_primitive(self, prim_name) -> bool:
        """Check if the cell is an instance of the primitive `prim_name`."""
        return (
            isinstance(self._cell.comp, ast.CompInst)
            and self._cell.comp.id == prim_name
        )

    def is_comb_mem_d1(self) -> bool:
        """Check if the cell is a StdMemD1 cell."""
        return self.is_primitive("comb_mem_d1")

    def is_seq_mem_d1(self) -> bool:
        """Check if the cell is a SeqMemD1 cell."""
        return self.is_primitive("seq_mem_d1")

    def infer_width_reg(self) -> int:
        """Infer the width of a register. That is, the width of `reg.in`."""
        assert self._cell.comp.id == "std_reg", "Cell is not a register"
        return self._cell.comp.args[0]

    def infer_width(self, port_name) -> int:
        """Infer the width of a port on the cell."""
        inst = self._cell.comp
        prim = inst.id
        if prim == "std_reg":
            if port_name in ("in", "out"):
                return inst.args[0]
            if port_name == "write_en":
                return 1
            return None
        # XXX(Caleb): add all the primitive names instead of adding whenever I need one
        if prim in (
            "std_add",
            "std_sub",
            "std_lt",
            "std_le",
            "std_ge",
            "std_gt",
            "std_eq",
            "std_neq",
            "std_sgt",
            "std_slt",
            "std_fp_sgt",
            "std_fp_slt",
        ):
            if port_name in ("left", "right"):
                return inst.args[0]
        if prim in ("comb_mem_d1", "seq_mem_d1"):
            if port_name == "write_en":
                return 1
            if port_name == "addr0":
                return inst.args[2]
            if port_name == "in":
                return inst.args[0]
            if prim == "seq_mem_d1" and port_name == "content_en":
                return 1
        if prim in (
            "std_mult_pipe",
            "std_smult_pipe",
            "std_mod_pipe",
            "std_smod_pipe",
            "std_div_pipe",
            "std_sdiv_pipe",
            "std_fp_smult_pipe",
        ):
            if port_name in ("left", "right"):
                return inst.args[0]
            if port_name == "go":
                return 1
        if prim == "std_wire" and port_name == "in":
            return inst.args[0]

        # Give up.
        return None

    @property
    def name(self) -> str:
        """Get the name of the cell."""
        return self._cell.id.name

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
                raise WidthInferenceError(f"could not infer width for literal {rhs}")
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
            raise MalformedGroupError(
                "GroupBuilder represents continuous assignments and does "
                "not have a done hole"
            )
        self.asgn(self.done, expr)

    def __enter__(self):
        TLS.groups.append(self)
        return self

    def __exit__(self, exc, value, tb):
        TLS.groups.pop()

    def infer_width(self, expr):
        """Try to guess the width of a port expression in this group."""
        assert isinstance(expr, ast.Atom)
        if isinstance(expr.item, ast.ThisPort):
            return self.comp.port_width(expr)
        cell_name = expr.item.id.name
        port_name = expr.item.name

        cell_builder = self.comp.index[cell_name]
        if not isinstance(cell_builder, CellBuilder):
            return None

        return cell_builder.infer_width(port_name)


def const(width: int, value: int) -> ExprBuilder:
    """Build a sized integer constant expression.

    This is available as a shorthand in cases where automatic width
    inference fails. Otherwise, you can just use plain Python integer
    values.
    """
    return ExprBuilder(ast.Atom(ast.ConstantPort(width, value)))


def infer_width(expr):
    """Try to guess the width of a port expression.

    Return an int, or None if we don't have a guess.
    """

    # Deal with `done` holes.
    expr = ExprBuilder.unwrap(expr)
    if isinstance(expr, ast.HolePort):
        assert expr.name == "done", f"unknown hole {expr.name}"
        return 1

    assert TLS.groups, "int width inference only works inside `with group:`"
    group_builder: GroupBuilder = TLS.groups[-1]

    return group_builder.infer_width(expr)


def ctx_asgn(lhs: ExprBuilder, rhs: Union[ExprBuilder, CondExprBuilder]):
    """Add an assignment to the current group context."""
    assert TLS.groups, "assignment outside `with group`"
    group_builder: GroupBuilder = TLS.groups[-1]
    group_builder.asgn(lhs, rhs)


LO = const(1, 0)  # A one bit low signal
HI = const(1, 1)  # A one bit high signal


def par(*args) -> ast.ParComp:
    """Build a parallel composition of control expressions.

    Each argument will become its own parallel arm in the resulting composition.
    So `par([a,b])` becomes `par {seq {a; b;}}` while `par(a, b)` becomes `par {a; b;}`.
    """
    return ast.ParComp([as_control(x) for x in args])


def static_par(*args) -> ast.StaticParComp:
    """Build a static parallel composition of control expressions.

    Each argument will become its own parallel arm in the resulting composition.
    So `par([a,b])` becomes `par {seq {a; b;}}` while `par(a, b)` becomes `par {a; b;}`.
    """
    return ast.StaticParComp([as_control(x) for x in args])


def seq(*args) -> ast.SeqComp:
    """Build a sequential composition of control expressions.

    Prefer use of python list syntax over this function. Use only when not directly
    modifying the control program with the `+=` operator.
    Each argument will become its own sequential arm in the resulting composition.
    So `seq([a,b], c)` becomes `seq { seq {a; b;} c }` while `seq(a, b, c)` becomes `seq
    {a; b; c;}`.
    """
    return ast.SeqComp([as_control(x) for x in args])


def static_seq(*args) -> ast.StaticSeqComp:
    """Build a static sequential composition of control expressions.

    Prefer use of python list syntax over this function. Use only when not directly
    modifying the control program with the `+=` operator.
    Each argument will become its own sequential arm in the resulting composition.
    So `seq([a,b], c)` becomes `seq { seq {a; b;} c }` while `seq(a, b, c)` becomes `seq
    {a; b; c;}`.
    """
    return ast.StaticSeqComp([as_control(x) for x in args])


def add_comp_params(comp: ComponentBuilder, input_ports: List, output_ports: List):
    """
    Adds `input_ports`/`output_ports` as inputs/outputs to comp.
    `input_ports`/`output_ports` should contain an (input_name, input_width) pair.
    """
    for name, width in input_ports:
        comp.input(name, width)
    for name, width in output_ports:
        comp.output(name, width)


def add_read_mem_params(comp: ComponentBuilder, name, data_width, addr_width):
    """
    Add parameters to component `comp` if we want to read from a mem named
    `name` with address width of `addr_width` and data width of `data_width`.
    """
    add_comp_params(
        comp,
        input_ports=[(f"{name}_read_data", data_width)],
        output_ports=[(f"{name}_addr0", addr_width)],
    )


def add_write_mem_params(comp: ComponentBuilder, name, data_width, addr_width):
    """
    Add arguments to component `comp` if we want to write to a mem named
    `name` with address width of `addr_width` and data width of `data_width`.
    """
    add_comp_params(
        comp,
        input_ports=[(f"{name}_done", 1)],
        output_ports=[
            (f"{name}_addr0", addr_width),
            (f"{name}_write_data", data_width),
            (f"{name}_write_en", 1),
        ],
    )


def add_register_params(comp: ComponentBuilder, name, width):
    """
    Add params to component `comp` if we want to use a register named `name`.
    """
    add_comp_params(
        comp,
        input_ports=[(f"{name}_done", 1), (f"{name}_out", width)],
        output_ports=[
            (f"{name}_in", width),
            (f"{name}_write_en", 1),
        ],
    )


def build_connections(
    cell1: Union[CellBuilder, ThisBuilder],
    cell2: Union[CellBuilder, ThisBuilder],
    root1: str,
    root2: str,
    forward_ports: List,
    reverse_ports: List,
):
    """
    Intended for wiring together two cells whose ports have similar names.
    For each `name` in `forward_port_names`, adds the following connection:
    `(cell1.root1name, cell2.root2name)`
    For each `name` in `backwards_port_names`, adds the following connection:
    `(cell2.root2name, cell1.root1name)`
    `root1name` refers to the string formed by `root1 + name` (i.e., no underscore
    between root1 and name)
    Returns a list of the resulting connections
    """
    res = []
    for port in forward_ports:
        res.append((cell1.port(root1 + port), cell2.port(root2 + port)))
    for port in reverse_ports:
        res.append((cell2.port(root2 + port), cell1.port(root1 + port)))
    return res
