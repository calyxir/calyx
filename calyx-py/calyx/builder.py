from __future__ import annotations

import threading
import random
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
            imports=[],
            components=[],
        )
        self.imported = set()
        self.import_("primitives/core.futil")
        self._index: Dict[str, ComponentBuilder] = {}

    def component(self, name: str, cells=None, latency=None) -> ComponentBuilder:
        """Create a new component builder."""
        cells = cells or []
        comp_builder = ComponentBuilder(self, name, cells, latency)
        self.program.components.append(comp_builder.component)
        self._index[name] = comp_builder
        return comp_builder

    def get_component(self, name: str) -> ComponentBuilder:
        """Retrieve a component builder by name."""
        comp_builder = self._index.get(name)
        if comp_builder is None:
            raise Exception(f"Component `{name}' not found in program.")
        else:
            return comp_builder

    def import_(self, filename: str):
        """Add an `import` statement to the program."""
        if filename not in self.imported:
            self.imported.add(filename)
            self.program.imports.append(ast.Import(filename))


class ComponentBuilder:
    """Builds Calyx components definitions."""

    def __init__(
        self,
        prog: Builder,
        name: str,
        cells: Optional[List[ast.Cell]] = None,
        latency: Optional[int] = None,
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
            latency=latency,
        )
        self.index: Dict[str, Union[GroupBuilder, CellBuilder]] = {}
        for cell in cells:
            self.index[cell.id.name] = CellBuilder(cell)
        self.continuous = GroupBuilder(None, self)

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

    def get_port_width(self, name: str) -> int:
        for input in self.component.inputs:
            if input.id.name == name:
                return input.width
        for output in self.component.outputs:
            if output.id.name == name:
                return output.width
        raise Exception(f"couldn't find port {name} on component {self.component.name}")

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

    def reg(self, name: str, size: int, is_ref=False) -> CellBuilder:
        """Generate a StdReg cell."""
        return self.cell(name, ast.Stdlib.register(size), False, is_ref)

    def wire(self, name: str, size: int, is_ref=False) -> CellBuilder:
        """Generate a StdReg cell."""
        return self.cell(name, ast.Stdlib.wire(size), False, is_ref)

    def slice(self, name: str, in_width: int, out_width, is_ref=False) -> CellBuilder:
        """Generate a StdReg cell."""
        return self.cell(name, ast.Stdlib.slice(in_width, out_width), False, is_ref)

    def const(self, name: str, width: int, value: int) -> CellBuilder:
        """Generate a StdConstant cell."""
        return self.cell(name, ast.Stdlib.constant(width, value))

    def mem_d1(
        self,
        name: str,
        bitwidth: int,
        len: int,
        idx_size: int,
        is_external=False,
        is_ref=False,
    ) -> CellBuilder:
        """Generate a StdMemD1 cell."""
        return self.cell(
            name, ast.Stdlib.mem_d1(bitwidth, len, idx_size), is_external, is_ref
        )

    def seq_mem_d1(
        self,
        name: str,
        bitwidth: int,
        len: int,
        idx_size: int,
        is_external=False,
        is_ref=False,
    ) -> CellBuilder:
        """Generate a SeqMemD1 cell."""
        self.prog.import_("primitives/memories.futil")
        return self.cell(
            name, ast.Stdlib.seq_mem_d1(bitwidth, len, idx_size), is_external, is_ref
        )

    def add(self, size: int, name=None, signed=False) -> CellBuilder:
        """Generate a StdAdd cell."""
        self.prog.import_("primitives/binary_operators.futil")
        name = name or f"add_{random.randint(0, 2**32)}"
        assert isinstance(name, str)
        return self.cell(name, ast.Stdlib.op("add", size, signed))

    def sub(self, size: int, name=None, signed=False) -> CellBuilder:
        """Generate a StdSub cell."""
        self.prog.import_("primitives/binary_operators.futil")
        name = name or f"sub_{random.randint(0, 2**32)}"
        assert isinstance(name, str)
        return self.cell(name, ast.Stdlib.op("sub", size, signed))

    def gt(self, size: int, name=None, signed=False) -> CellBuilder:
        """Generate a StdGt cell."""
        self.prog.import_("primitives/binary_operators.futil")
        name = name or f"gt_{random.randint(0, 2**32)}"
        assert isinstance(name, str)
        return self.cell(name, ast.Stdlib.op("gt", size, signed))

    def lt(self, size: int, name=None, signed=False) -> CellBuilder:
        """Generate a StdLt cell."""
        self.prog.import_("primitives/binary_operators.futil")
        name = name or f"lt_{random.randint(0, 2**32)}"
        assert isinstance(name, str)
        return self.cell(name, ast.Stdlib.op("lt", size, signed))

    def eq(self, size: int, name=None, signed=False) -> CellBuilder:
        """Generate a StdEq cell."""
        self.prog.import_("primitives/binary_operators.futil")
        name = name or f"eq_{random.randint(0, 2**32)}"
        assert isinstance(name, str)
        return self.cell(name, ast.Stdlib.op("eq", size, signed))

    def neq(self, size: int, name=None, signed=False) -> CellBuilder:
        """Generate a StdNeq cell."""
        self.prog.import_("primitives/binary_operators.futil")
        name = name or f"neq_{random.randint(0, 2**32)}"
        assert isinstance(name, str)
        return self.cell(name, ast.Stdlib.op("neq", size, signed))

    def ge(self, size: int, name=None, signed=False) -> CellBuilder:
        """Generate a StdGe cell."""
        self.prog.import_("primitives/binary_operators.futil")
        name = name or f"ge_{random.randint(0, 2**32)}"
        assert isinstance(name, str)
        return self.cell(name, ast.Stdlib.op("ge", size, signed))

    def le(self, size: int, name=None, signed=False) -> CellBuilder:
        """Generate a StdLe cell."""
        self.prog.import_("primitives/binary_operators.futil")
        name = name or f"le_{random.randint(0, 2**32)}"
        assert isinstance(name, str)
        return self.cell(name, ast.Stdlib.op("le", size, signed))

    def and_(self, size: int, name=None) -> CellBuilder:
        """Generate a StdAnd cell."""
        name = name or f"and_{random.randint(0, 2**32)}"
        assert isinstance(name, str)
        return self.cell(name, ast.Stdlib.op("and", size, False))

    def not_(self, size: int, name=None) -> CellBuilder:
        """Generate a StdNot cell."""
        name = name or f"not_{random.randint(0, 2**32)}"
        assert isinstance(name, str)
        return self.cell(name, ast.Stdlib.op("not", size, False))

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

    def insert_comb_group(self, left, right, cell, groupname=None):
        """Accepts a cell that performs some computation on values {left} and {right}.
        Creates a combinational group that wires up the cell with these ports.
        Returns the cell and the combintational group.
        """
        groupname = groupname or f"{cell.name()}_group"
        with self.comb_group(groupname) as comb_group:
            cell.left = left
            cell.right = right
        return CellGroupBuilder(cell, comb_group)

    def eq_use(self, left, right, width, cellname=None):
        """Inserts wiring into component {self} to check if {left} == {right}.

        <cellname> = std_eq(<width>);
        ...
        comb group <cellname>_group {
            <cellname>.left = <left>;
            <cellname>.right = <right>;
        }

        Returns handles to the cell and the combinational group.
        """
        return self.insert_comb_group(left, right, self.eq(width, cellname))

    def neq_use(self, left, right, width, cellname=None):
        """Inserts wiring into component {self} to check if {left} != {right}.

        <cellname> = std_neq(<width>);
        ...
        comb group <cellname>_group {
            <cellname>.left = <left>;
            <cellname>.right = <right>;
        }

        Returns handles to the cell and the combinational group.
        """
        return self.insert_comb_group(left, right, self.neq(width, cellname))

    def lt_use(self, left, right, width, cellname=None):
        """Inserts wiring into component {self} to check if {left} < {right}.

        <cellname> = std_lt(<width>);
        ...
        comb group <cellname>_group {
            <cellname>.left = <left>;
            <cellname>.right = <right>;
        }

        Returns handles to the cell and the combinational group.
        """
        return self.insert_comb_group(left, right, self.lt(width, cellname))

    def le_use(self, left, right, width, cellname=None):
        """Inserts wiring into component {self} to check if {left} <= {right}.

        <cellname> = std_le(<width>);
        ...
        comb group <cellname>_group {
            <cellname>.left = <left>;
            <cellname>.right = <right>;
        }

        Returns handles to the cell and the combinational group.
        """
        return self.insert_comb_group(left, right, self.le(width, cellname))

    def ge_use(self, left, right, width, cellname=None):
        """Inserts wiring into component {self} to check if {left} >= {right}.

        <cellname> = std_ge(<width>);
        ...
        comb group <cellname>_group {
            <cellname>.left = <left>;
            <cellname>.right = <right>;
        }

        Returns handles to the cell and the combinational group.
        """
        return self.insert_comb_group(left, right, self.ge(width, cellname))

    def gt_use(self, left, right, width, cellname=None):
        """Inserts wiring into component {self} to check if {left} > {right}.

        <cellname> = std_gt(<width>);
        ...
        comb group <cellname>_group {
            <cellname>.left = <left>;
            <cellname>.right = <right>;
        }

        Returns handles to the cell and the combinational group.
        """
        return self.insert_comb_group(left, right, self.gt(width, cellname))

    def add_use(self, left, right, width, cellname=None):
        """Inserts wiring into component {self} to compute {left} + {right}.

        <cellname> = std_add(<width>);
        ...
        comb group <cellname>_group {
            <cellname>.left = <left>;
            <cellname>.right = <right>;
        }

        Returns handles to the cell and the combinational group.
        """
        return self.insert_comb_group(left, right, self.add(width, cellname))

    def sub_use(self, left, right, width, cellname=None):
        """Inserts wiring into component {self} to compute {left} - {right}.

        <cellname> = std_sub(<width>);
        ...
        comb group <cellname>_group {
            <cellname>.left = <left>;
            <cellname>.right = <right>;
        }

        Returns handles to the cell and the combinational group.
        """
        return self.insert_comb_group(left, right, self.sub(width, cellname))

    def bitwise_flip_reg(self, reg, width, cellname=None):
        """Inserts wiring into component {self} to bitwise-flip the contents of {reg}.

        Returns a handle to the group that does this.
        """
        cellname = cellname or f"{reg.name()}_not"
        not_cell = self.not_(width, cellname)
        with self.group(f"{cellname}_group") as not_group:
            not_cell.in_ = reg.out
            reg.write_en = 1
            reg.in_ = not_cell.out
            not_group.done = reg.done
        return not_group

    def incr(self, reg, width, val=1, cellname=None):
        """Inserts wiring into component {self} to increment register {reg} by {val}.
        1. Within component {self}, creates a group.
        2. Within the group, adds a cell {cellname} that computes sums.
        3. Puts the values {reg} and {val} into the cell.
        4. Then puts the answer of the computation back into {reg}.
        5. Returns the group that does this.
        """
        cellname = cellname or f"{reg.name()}_incr"
        add_cell = self.add(width, cellname)
        with self.group(f"{cellname}_group") as incr_group:
            add_cell.left = reg.out
            add_cell.right = const(32, val)
            reg.write_en = 1
            reg.in_ = add_cell.out
            incr_group.done = reg.done
        return incr_group

    def decr(self, reg, width, val=1, cellname=None):
        """Inserts wiring into component {self} to decrement register {reg} by {val}.
        1. Within component {self}, creates a group.
        2. Within the group, adds a cell {cellname} that computes differences.
        3. Puts the values {reg} and {val} into the cell.
        4. Then puts the answer of the computation back into {reg}.
        5. Returns the group.
        """
        cellname = cellname or f"{reg.name()}_decr"
        sub_cell = self.sub(width, cellname)
        with self.group(f"{cellname}_group") as decr_group:
            sub_cell.left = reg.out
            sub_cell.right = const(32, val)
            reg.write_en = 1
            reg.in_ = sub_cell.out
            decr_group.done = reg.done
        return decr_group

    def reg_store(self, reg, val, groupname=None):
        """Stores a value in a register.
        1. Within component {self}, creates a group.
        2. Within the group, sets the register {reg} to {val}.
        3. Returns the group.
        """
        groupname = groupname or f"{reg.name()}_store_to_reg"
        with self.group(groupname) as reg_grp:
            reg.in_ = val
            reg.write_en = 1
            reg_grp.done = reg.done
        return reg_grp

    def mem_load_std_d1(self, mem, i, reg, groupname=None):
        """Loads a value from one memory (std_d1) into a register.
        1. Within component {comp}, creates a group.
        2. Within the group, reads from memory {mem} at address {i}.
        3. Writes the value into register {reg}.
        4. Returns the group.
        """
        assert mem.is_std_mem_d1()
        groupname = groupname or f"{mem.name()}_load_to_reg"
        with self.group(groupname) as load_grp:
            mem.addr0 = i
            reg.write_en = 1
            reg.in_ = mem.read_data
            load_grp.done = reg.done
        return load_grp

    def mem_store_std_d1(self, mem, i, val, groupname=None):
        """Stores a value into a (std_d1) memory.
        1. Within component {self}, creates a group.
        2. Within the group, reads from {val}.
        3. Writes the value into memory {mem} at address i.
        4. Returns the group.
        """
        assert mem.is_std_mem_d1()
        groupname = groupname or f"store_into_{mem.name()}"
        with self.group(groupname) as store_grp:
            mem.addr0 = i
            mem.write_en = 1
            mem.write_data = val
            store_grp.done = mem.done
        return store_grp

    def mem_read_seq_d1(self, mem, i, groupname=None):
        """Given a seq_mem_d1, reads from memory at address i.
        Note that this does not write the value anywhere.

        1. Within component {self}, creates a group.
        2. Within the group, reads from memory {mem} at address {i},
        thereby "latching" the value.
        3. Returns the group.
        """
        assert mem.is_seq_mem_d1()
        groupname = groupname or f"read_from_{mem.name()}"
        with self.group(groupname) as read_grp:
            mem.addr0 = i
            mem.read_en = 1
            read_grp.done = mem.read_done
        return read_grp

    def mem_write_seq_d1_to_reg(self, mem, reg, groupname=None):
        """Given a seq_mem_d1 that is already assumed to have a latched value,
        reads the latched value and writes it to a register.

        1. Within component {self}, creates a group.
        2. Within the group, reads from memory {mem}.
        3. Writes the value into register {reg}.
        4. Returns the group.
        """
        assert mem.is_seq_mem_d1()
        groupname = groupname or f"{mem.name()}_write_to_reg"
        with self.group(groupname) as write_grp:
            reg.write_en = 1
            reg.in_ = mem.read_data
            write_grp.done = reg.done
        return write_grp

    def mem_store_seq_d1(self, mem, i, val, groupname=None):
        """Given a seq_mem_d1, stores a value into memory at address i.

        1. Within component {self}, creates a group.
        2. Within the group, reads from {val}.
        3. Writes the value into memory {mem} at address i.
        4. Returns the group.
        """
        assert mem.is_seq_mem_d1()
        groupname = groupname or f"{mem.name()}_store"
        with self.group(groupname) as store_grp:
            mem.addr0 = i
            mem.write_en = 1
            mem.write_data = val
            store_grp.done = mem.write_done
        return store_grp

    def mem_load_to_mem(self, mem, i, ans, j, groupname=None):
        """Loads a value from one std_mem_d1 memory into another.
        1. Within component {self}, creates a group.
        2. Within the group, reads from memory {mem} at address {i}.
        3. Writes the value into memory {ans} at address {j}.
        4. Returns the group.
        """
        assert mem.is_std_mem_d1() and ans.is_std_mem_d1()
        groupname = groupname or f"{mem.name()}_load_to_mem"
        with self.group(groupname) as load_grp:
            mem.addr0 = i
            ans.write_en = 1
            ans.addr0 = j
            ans.write_data = mem.read_data
            load_grp.done = ans.done
        return load_grp

    def add_store_in_reg(self, cellname, left, right, ans_reg=None):
        """Inserts wiring into component {self} to compute {left} + {right} and
        store it in {ans_reg}.
        1. Within component {self}, creates a group called {cellname}_group.
        2. Within {group}, create a cell {cellname} that computes sums.
        3. Puts the values of {left} and {right} into the cell.
        4. Then puts the answer of the computation into {ans_reg}.
        4. Returns the summing group and the register.
        """
        add_cell = self.add(32, cellname)
        ans_reg = ans_reg or self.reg(f"reg_{cellname}", 32)
        with self.group(f"{cellname}_group") as adder_group:
            add_cell.left = left
            add_cell.right = right
            ans_reg.write_en = 1
            ans_reg.in_ = add_cell.out
            adder_group.done = ans_reg.done
        return adder_group, ans_reg

    def sub_store_in_reg(self, left, right, cellname, width, ans_reg=None):
        """Adds wiring into component {self} to compute {left} - {right}
        and store it in {ans_reg}.
        1. Within component {self}, creates a group called {cellname}_group.
        2. Within {group}, create a cell {cellname} that computes differences.
        3. Puts the values of {left} and {right} into {cell}.
        4. Then puts the answer of the computation into {ans_reg}.
        4. Returns the subtracting group and the register.
        """
        sub_cell = self.sub(width, cellname)
        ans_reg = ans_reg or self.reg(f"reg_{cellname}", width)
        with self.group(f"{cellname}_group") as sub_group:
            sub_cell.left = left
            sub_cell.right = right
            ans_reg.write_en = 1
            ans_reg.in_ = sub_cell.out
            sub_group.done = ans_reg.done
        return sub_group, ans_reg


class CellGroupBuilder:
    """Just a cell and a group, for when it is convenient to
    pass them around together.

    Typically the group will be a combinational group, and `if_with` and
    `while_with` will require that a CellGroupBuilder be passed in, not a
    cell and a group separately.
    """

    def __init__(self, cell, group):
        self.cell = cell
        self.group = group


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
    """Build a `static if` control statement.

    To build an `if` statement with a combinational group, use `if_with`.
    """
    else_body = else_body or ast.Empty()
    return ast.If(port.expr, None, as_control(body), as_control(else_body))


def static_if(
    port: ExprBuilder,
    body,
    else_body=None,
) -> ast.If:
    """Build an `if` control statement."""
    else_body = else_body or ast.Empty()
    return ast.StaticIf(port.expr, as_control(body), as_control(else_body))


def if_with(port_comb: CellGroupBuilder, body, else_body=None) -> ast.If:
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


def while_with(port_comb: CellGroupBuilder, body) -> ast.While:
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

    def is_primitive(self, prim_name) -> bool:
        """Check if the cell is an instance of the primitive {prim_name}."""
        return (
            isinstance(self._cell.comp, ast.CompInst)
            and self._cell.comp.id == prim_name
        )

    def is_std_mem_d1(self) -> bool:
        """Check if the cell is a StdMemD1 cell."""
        return self.is_primitive("std_mem_d1")

    def is_seq_mem_d1(self) -> bool:
        """Check if the cell is a SeqMemD1 cell."""
        return self.is_primitive("seq_mem_d1")

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
    assert TLS.groups, "int width inference only works inside `with group:`"
    group_builder: GroupBuilder = TLS.groups[-1]

    # Deal with `done` holes.
    expr = ExprBuilder.unwrap(expr)
    if isinstance(expr, ast.HolePort):
        assert expr.name == "done", f"unknown hole {expr.name}"
        return 1

    # Otherwise, it's a `cell.port` lookup.
    assert isinstance(expr, ast.Atom)
    if isinstance(expr.item, ast.ThisPort):
        name = expr.item.id.name
        return group_builder.comp.get_port_width(name)
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
    # XXX(Caleb): add all the primitive names instead of adding whenever I need one
    elif prim in (
        "std_add",
        "std_lt",
        "std_le",
        "std_ge",
        "std_gt",
        "std_eq",
        "std_sgt",
        "std_slt",
        "std_fp_sgt",
        "std_fp_slt",
    ):
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
    elif prim in (
        "std_mult_pipe",
        "std_smult_pipe",
        "std_mod_pipe",
        "std_smod_pipe",
        "std_div_pipe",
        "std_sdiv_pipe",
        "std_fp_smult_pipe",
    ):
        if port_name == "left" or port_name == "right":
            return inst.args[0]
        elif port_name == "go":
            return 1
    elif prim == "std_wire":
        if port_name == "in":
            return inst.args[0]

    # Give up.
    return None


def ctx_asgn(lhs: ExprBuilder, rhs: Union[ExprBuilder, CondExprBuilder]):
    """Add an assignment to the current group context."""
    assert TLS.groups, "assignment outside `with group`"
    group_builder: GroupBuilder = TLS.groups[-1]
    group_builder.asgn(lhs, rhs)


"""A one bit low signal"""
LO = const(1, 0)
"""A one bit high signal"""
HI = const(1, 1)


def par(*args) -> ast.ParComp:
    """Build a parallel composition of control expressions.

    Each argument will become its own parallel arm in the resulting composition.
    So `par([a,b])` becomes `par {seq {a; b;}}` while `par(a, b)` becomes `par {a; b;}`.
    """
    return ast.ParComp([as_control(x) for x in args])


def seq(*args) -> ast.SeqComp:
    """Build a sequential composition of control expressions.

    Prefer use of python list syntax over this function. Use only when not directly
    modifying the control program with the `+=` operator.
    Each argument will become its own sequential arm in the resulting composition.
    So `seq([a,b], c)` becomes `seq { seq {a; b;} c }` while `seq(a, b, c)` becomes `seq
    {a; b; c;}`.
    """
    return ast.SeqComp([as_control(x) for x in args])
