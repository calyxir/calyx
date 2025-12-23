from __future__ import annotations  # Used for circular dependencies.
from dataclasses import dataclass, field
from typing import Dict, List, Any, Tuple, Optional
from calyx.utils import block
import inspect
import os

"""
The base path that fileinfos will be relative with respect to. If None, absolute paths will be used.
"""
FILEINFO_BASE_PATH = None

"""
Determines if sourceinfo and @pos attributes should be emitted or not.
"""
EMIT_SOURCELOC = True


@dataclass
class Emittable:
    def doc(self) -> str:
        assert False, f"`doc` not implemented for {type(self).__name__}"

    def emit(self):
        print(self.doc())


class FileTable:
    # global counter to ensure unique ids
    counter: int = 0
    table: Dict[str, int] = {}

    @staticmethod
    def get_fileid(filename):
        if filename not in FileTable.table:
            FileTable.table[filename] = FileTable.counter
            FileTable.counter += 1
        return FileTable.table[filename]

    @staticmethod
    def emit_fileinfo_metadata():
        contents = []
        for filename, fileid in FileTable.table.items():
            contents.append(f"{fileid}: {filename}")
        return block("FILES", contents, with_curly=False)


class PosTable:
    counter: int = 0
    table: Dict[Tuple[int, int], int] = {}  # contents: (fileid, linenum) -> positionId

    @staticmethod
    def determine_source_loc() -> Optional[int]:
        """Inspects the call stack to determine the first call site outside the calyx-py library."""
        if not EMIT_SOURCELOC:
            return None

        stacktrace = inspect.stack()

        # inspect top frame to determine the path to the calyx-py library
        top = stacktrace[0]
        assert top.function == "determine_source_loc"
        library_path = os.path.dirname(top.filename)
        assert os.path.join(library_path, "py_ast.py") == top.filename

        # find first stack frame that is not part of the library
        user = None
        for frame in stacktrace:
            # skip frames that do not have a real filename
            if frame.filename == "<string>":
                continue
            if not frame.filename.startswith(library_path):
                user = frame
                break
        if user is None:
            return None

        # filename depends on whether we're testing or not.
        if FILEINFO_BASE_PATH is None:
            filename = frame.filename
        else:
            filename = os.path.relpath(frame.filename, FILEINFO_BASE_PATH)

        return PosTable.add_entry(filename, frame.lineno)

    @staticmethod
    def add_entry(filename, line_num):
        file_id = FileTable.get_fileid(filename)
        if (file_id, line_num) not in PosTable.table:
            PosTable.table[(file_id, line_num)] = PosTable.counter
            PosTable.counter += 1
        return PosTable.table[(file_id, line_num)]

    @staticmethod
    def emit_fileinfo_metadata():
        contents = []
        for (fileid, linenum), position_id in PosTable.table.items():
            contents.append(f"{position_id}: {fileid} {linenum}")
        return block("POSITIONS", contents, with_curly=False)


# Program
@dataclass
class Import(Emittable):
    filename: str

    def doc(self) -> str:
        return f'import "{self.filename}";'


@dataclass
class Program(Emittable):
    imports: List[Import]
    components: List[Component]
    meta: dict[Any, str] = field(default_factory=dict)

    def doc(self) -> str:
        out = "\n".join([i.doc() for i in self.imports])
        if len(self.imports) > 0:
            out += "\n"
        out += "\n".join([c.doc() for c in self.components])
        if len(self.meta) > 0:
            out += "\nmetadata #{\n"
            for key, val in self.meta.items():
                out += f"{key}: {val}\n"
            out += "}#"
        if EMIT_SOURCELOC and len(FileTable.table) > 0 and len(PosTable.table) > 0:
            out += "\n\nsourceinfo #{\n"
            out += FileTable.emit_fileinfo_metadata()
            out += PosTable.emit_fileinfo_metadata()
            out += "}#"

        return out


# Component
@dataclass
class Component:
    name: str
    attributes: set[Attribute]
    inputs: list[PortDef]
    outputs: list[PortDef]
    wires: list[Structure]
    cells: list[Cell]
    controls: Control
    latency: Optional[int]
    loc: Optional[int]

    def __init__(
        self,
        name: str,
        inputs: list[PortDef],
        outputs: list[PortDef],
        structs: list[Structure],
        controls: Control,
        attributes: Optional[set[CompAttribute]] = None,
        latency: Optional[int] = None,
    ):
        self.name = name
        self.attributes = attributes
        self.inputs = inputs
        self.outputs = outputs
        self.controls = controls
        self.latency = latency
        self.loc = PosTable.determine_source_loc()

        # Partition cells and wires.
        def is_cell(x):
            return isinstance(x, Cell)

        self.cells = [s for s in structs if is_cell(s)]
        self.wires = [s for s in structs if not is_cell(s)]

    def get_cell(self, name: str) -> Cell:
        for cell in self.cells:
            if cell.id.name == name:
                return cell
        raise Exception(
            f"Cell `{name}' not found in component {self.name}. Currently defined cells: {[c.id.name for c in self.cells]}"
        )

    def doc(self) -> str:
        # if(self.name == "axi_seq_mem_A0"):
        #     for s in self.inputs:
        #         print(s.doc())
        ins = ", ".join([s.doc() for s in self.inputs])
        outs = ", ".join([s.doc() for s in self.outputs])
        latency_annotation = (
            f"static<{self.latency}> " if self.latency is not None else ""
        )
        if self.loc is not None:
            loc_attribute = CompAttribute("pos", self.loc)
            if self.attributes:
                self.attributes.add(loc_attribute)
            else:
                self.attributes = {loc_attribute}
        attribute_annotation = (
            f"<{', '.join([f'{a.doc()}' for a in sorted(self.attributes, key=lambda a: a.name)])}>"
            if self.attributes
            else ""
        )
        signature = f"{latency_annotation}component {self.name}{attribute_annotation}({ins}) -> ({outs})"
        cells = block("cells", [c.doc() for c in self.cells])
        wires = block("wires", [w.doc() for w in self.wires])
        controls = block("control", [self.controls.doc()])
        return block(signature, [cells, wires, controls])


# CombComponent
@dataclass
class CombComponent:
    """Like a Component, but with no latency and no control."""

    name: str
    attributes: set[Attribute]
    inputs: list[PortDef]
    outputs: list[PortDef]
    wires: list[Structure]
    cells: list[Cell]
    loc: Optional[int]

    def __init__(
        self,
        name: str,
        inputs: list[PortDef],
        outputs: list[PortDef],
        structs: list[Structure],
        attributes: Optional[set[CompAttribute]] = None,
    ):
        self.name = name
        self.attributes = attributes
        self.inputs = inputs
        self.outputs = outputs
        self.loc = PosTable.determine_source_loc()

        # Partition cells and wires.
        def is_cell(x):
            return isinstance(x, Cell)

        self.cells = [s for s in structs if is_cell(s)]
        self.wires = [s for s in structs if not is_cell(s)]

    def get_cell(self, name: str) -> Cell:
        for cell in self.cells:
            if cell.id.name == name:
                return cell
        raise Exception(
            f"Cell `{name}' not found in component {self.name}. Currently defined cells: {[c.id.name for c in self.cells]}"
        )

    def doc(self) -> str:
        ins = ", ".join([s.doc() for s in self.inputs])
        outs = ", ".join([s.doc() for s in self.outputs])
        attribute_annotation = (
            f"<{', '.join([f'{a.doc()}' for a in sorted(self.attributes, key=lambda a: a.name)])}>"
            if self.attributes
            else ""
        )
        signature = (
            f"comb component {self.name}{attribute_annotation}({ins}) -> ({outs})"
        )
        cells = block("cells", [c.doc() for c in self.cells])
        wires = block("wires", [w.doc() for w in self.wires])
        return block(signature, [cells, wires])


# Attribute
@dataclass
class Attribute(Emittable):
    pass


@dataclass
class CompAttribute(Attribute):
    name: str
    value: int

    def __hash__(self):
        return hash((self.name, self.value))

    def doc(self) -> str:
        if self.name == "pos":
            return f'"{self.name}"={{{self.value}}}'
        else:
            return f'"{self.name}"={self.value}'


@dataclass
class CellAttribute(Attribute):
    name: str
    value: Optional[int] = None

    def __hash__(self):
        return hash((self.name, self.value))

    def doc(self) -> str:
        if self.value is None:
            return f"@{self.name}"
        elif self.name == "pos":
            return f"@{self.name}{{{self.value}}}"
        else:
            return f"@{self.name}({self.value})"


@dataclass
class GroupAttribute(Attribute):
    name: str
    value: int

    def __hash__(self):
        return hash((self.name, self.value))

    def doc(self) -> str:
        if self.name == "pos":
            return f'"{self.name}"={{{self.value}}}'
        else:
            return f'"{self.name}"={self.value}'


@dataclass
class PortAttribute(Attribute):
    name: str
    value: Optional[int] = None

    def __hash__(self):
        return hash((self.name, self.value))

    def doc(self) -> str:
        return f"@{self.name}" if self.value is None else f"@{self.name}({self.value})"


# Ports
@dataclass
class Port(Emittable):
    pass


@dataclass
class CompPort(Port):
    id: CompVar
    name: str

    def doc(self) -> str:
        return f"{self.id.doc()}.{self.name}"

    def get_name(self) -> str:
        return f"{self.id.doc()}_{self.name}"


@dataclass
class ThisPort(Port):
    id: CompVar

    def doc(self) -> str:
        return self.id.doc()

    def get_name(self) -> str:
        return self.id.get_name()


@dataclass
class HolePort(Port):
    id: CompVar
    name: str

    def doc(self) -> str:
        return f"{self.id.doc()}[{self.name}]"

    def get_name(self) -> str:
        return self.name


@dataclass
class ConstantPort(Port):
    width: int
    value: int

    def doc(self) -> str:
        return f"{self.width}'d{self.value}"


@dataclass
class CompVar(Emittable):
    name: str

    def doc(self) -> str:
        return self.name

    def port(self, port: str) -> CompPort:
        return CompPort(self, port)

    def add_suffix(self, suffix: str) -> CompVar:
        return CompVar(f"{self.name}{suffix}")

    def get_name(self) -> str:
        return self.name


@dataclass
class PortDef(Emittable):
    id: CompVar
    width: int
    attributes: set[PortAttribute] = field(default_factory=set)

    def doc(self) -> str:
        attributes = (
            ""
            if len(self.attributes) == 0
            else (
                " ".join(
                    [x.doc() for x in sorted(self.attributes, key=lambda a: a.name)]
                )
                + " "
            )
        )
        return f"{attributes}{self.id.doc()}: {self.width}"


# Structure
@dataclass
class Structure(Emittable):
    pass


@dataclass
class Cell(Structure):
    id: CompVar
    comp: CompInst
    is_external: bool = False
    is_ref: bool = False
    attributes: set[CellAttribute] = field(default_factory=set)
    loc: Optional[int] = field(default_factory=PosTable.determine_source_loc)

    def doc(self) -> str:
        assert not (self.is_ref and self.is_external), (
            "Cell cannot be both a ref and external"
        )
        if self.is_external:
            self.attributes.add(CellAttribute("external"))
        if self.loc is not None:
            self.attributes.add(CellAttribute("pos", self.loc))
        attribute_annotation = (
            f"{' '.join([f'{a.doc()}' for a in sorted(self.attributes, key=lambda a: a.name)])} "
            if len(self.attributes) > 0
            else ""
        )
        ref = "ref " if self.is_ref else ""
        return f"{attribute_annotation}{ref}{self.id.doc()} = {self.comp.doc()};"


@dataclass
class Connect(Structure):
    dest: Port
    src: Port
    guard: Optional[GuardExpr] = None

    def doc(self) -> str:
        source = (
            self.src.doc()
            if self.guard is None
            else f"{self.guard.doc()} ? {self.src.doc()}"
        )
        return f"{self.dest.doc()} = {source};"


@dataclass
class Group(Structure):
    id: CompVar
    connections: list[Connect]
    # XXX: This is a static group now. Remove this and add a new StaticGroup class.
    static_delay: Optional[int] = None
    attributes: set[GroupAttribute] = field(default_factory=set)
    loc: Optional[int] = field(default_factory=PosTable.determine_source_loc)

    def doc(self) -> str:
        if self.static_delay is not None:
            self.attributes.add(GroupAttribute("promotable", self.static_delay))
        if self.loc is not None:
            self.attributes.add(GroupAttribute("pos", self.loc))
        attribute_annotation = (
            f"<{', '.join([f'{a.doc()}' for a in sorted(self.attributes, key=lambda a: a.name)])}>"
            if len(self.attributes) > 0
            else ""
        )
        return block(
            f"group {self.id.doc()}{attribute_annotation}",
            [c.doc() for c in self.connections],
        )


@dataclass
class CombGroup(Structure):
    id: CompVar
    connections: list[Connect]
    loc: Optional[int] = field(default_factory=PosTable.determine_source_loc)

    def doc(self) -> str:
        attribute_annotation = (
            f"<{GroupAttribute("pos", self.loc).doc()}>"
            if self.loc is not None 
            else ""
        )
        return block(
            f"comb group {self.id.doc()}{attribute_annotation}",
            [c.doc() for c in self.connections],
        )


@dataclass
class StaticGroup(Structure):
    id: CompVar
    connections: list[Connect]
    latency: int
    loc: Optional[int] = field(default_factory=PosTable.determine_source_loc)

    def doc(self) -> str:
        attribute_annotation = (
            f"<{GroupAttribute("pos", self.loc).doc()}>"
            if self.loc is not None 
            else ""
        )
        return block(
            f"static<{self.latency}> group {self.id.doc()}{attribute_annotation}",
            [c.doc() for c in self.connections],
        )


@dataclass
class CompInst(Emittable):
    id: str
    args: list[int]

    def doc(self) -> str:
        args = ", ".join([str(x) for x in self.args])
        return f"{self.id}({args})"


# Guard Expressions
@dataclass
class GuardExpr(Emittable):
    pass


@dataclass
class Atom(GuardExpr):
    item: Port

    def doc(self) -> str:
        return self.item.doc()

    @property
    def name(self) -> str:
        return f"{self.item.get_name()}"


@dataclass
class Not(GuardExpr):
    inner: GuardExpr

    def doc(self) -> str:
        return f"!{self.inner.doc()}"


@dataclass
class And(GuardExpr):
    left: GuardExpr
    right: GuardExpr

    def doc(self) -> str:
        return f"({self.left.doc()} & {self.right.doc()})"


@dataclass
class Or(GuardExpr):
    left: GuardExpr
    right: GuardExpr

    def doc(self) -> str:
        return f"({self.left.doc()} | {self.right.doc()})"


@dataclass
class Eq(GuardExpr):
    left: GuardExpr
    right: GuardExpr

    def doc(self) -> str:
        return f"({self.left.doc()} == {self.right.doc()})"


@dataclass
class Neq(GuardExpr):
    left: GuardExpr
    right: GuardExpr

    def doc(self) -> str:
        return f"({self.left.doc()} != {self.right.doc()})"


@dataclass
class Lt(GuardExpr):
    left: GuardExpr
    right: GuardExpr

    def doc(self) -> str:
        return f"({self.left.doc()} < {self.right.doc()})"


@dataclass
class Lte(GuardExpr):
    left: GuardExpr
    right: GuardExpr

    def doc(self) -> str:
        return f"({self.left.doc()} <= {self.right.doc()})"


@dataclass
class Gt(GuardExpr):
    left: GuardExpr
    right: GuardExpr

    def doc(self) -> str:
        return f"({self.left.doc()} > {self.right.doc()})"


@dataclass
class Gte(GuardExpr):
    left: GuardExpr
    right: GuardExpr

    def doc(self) -> str:
        return f"({self.left.doc()} >= {self.right.doc()})"


# Control
@dataclass
class Control(Emittable):
    pass


def ctrl_with_pos_attribute(source: str, loc: Optional[int]) -> str:
    """adds the @pos attribute of loc is not None"""
    if loc is None:
        return source
    else:
        return f"@pos{{{loc}}} {source}"


@dataclass
class Enable(Control):
    stmt: str
    loc: Optional[int] = field(default_factory=PosTable.determine_source_loc)

    def doc(self) -> str:
        return ctrl_with_pos_attribute(f"{self.stmt};", self.loc)


@dataclass
class SeqComp(Control):
    stmts: list[Control]
    loc: Optional[int] = field(default_factory=PosTable.determine_source_loc)

    def doc(self) -> str:
        return ctrl_with_pos_attribute(
            block("seq", [s.doc() for s in self.stmts]), self.loc
        )


@dataclass
class StaticSeqComp(Control):
    stmts: list[Control]
    loc: Optional[int] = field(default_factory=PosTable.determine_source_loc)

    def doc(self) -> str:
        return ctrl_with_pos_attribute(
            block("static seq", [s.doc() for s in self.stmts]), self.loc
        )


@dataclass
class ParComp(Control):
    stmts: list[Control]
    loc: Optional[int] = field(default_factory=PosTable.determine_source_loc)

    def doc(self) -> str:
        return ctrl_with_pos_attribute(
            block("par", [s.doc() for s in self.stmts]), self.loc
        )


@dataclass
class StaticParComp(Control):
    stmts: list[Control]
    loc: Optional[int] = field(default_factory=PosTable.determine_source_loc)

    def doc(self) -> str:
        return ctrl_with_pos_attribute(
            block("static par", [s.doc() for s in self.stmts]), self.loc
        )


@dataclass
class Invoke(Control):
    id: CompVar
    in_connects: List[Tuple[str, Port]]
    out_connects: List[Tuple[str, Port]]
    ref_cells: List[Tuple[str, CompVar]] = field(default_factory=list)
    comb_group: Optional[CompVar] = None
    attributes: set[Tuple[str, int]] = field(default_factory=set)
    loc: Optional[int] = field(default_factory=PosTable.determine_source_loc)

    def doc(self) -> str:
        inv = f"invoke {self.id.doc()}"

        # Add attributes if present
        if len(self.attributes) > 0:
            attrs = " ".join(
                [
                    f"@{tag}({val})"
                    for tag, val in sorted(self.attributes, key=lambda a: a[0])
                ]
            )
            inv = f"{attrs} {inv}"

        # Add ref cells if present
        if len(self.ref_cells) > 0:
            rcs = ", ".join([f"{n}={arg.doc()}" for (n, arg) in self.ref_cells])
            inv += f"[{rcs}]"

        # Inputs and outputs
        in_defs = ", ".join([f"{p}={a.doc()}" for p, a in self.in_connects])
        out_defs = ", ".join([f"{p}={a.doc()}" for p, a in self.out_connects])
        inv += f"({in_defs})({out_defs})"

        # Combinational group if present
        if self.comb_group is not None:
            inv += f" with {self.comb_group.doc()}"
        inv += ";"

        return ctrl_with_pos_attribute(inv, self.loc)

    def with_attr(self, key: str, value: int) -> Invoke:
        self.attributes.add((key, value))
        return self


@dataclass
class StaticInvoke(Control):
    id: CompVar
    in_connects: List[Tuple[str, Port]]
    out_connects: List[Tuple[str, Port]]
    ref_cells: List[Tuple[str, CompVar]] = field(default_factory=list)
    attributes: set[Tuple[str, int]] = field(default_factory=set)

    def doc(self) -> str:
        inv = f"static invoke {self.id.doc()}"

        # Add attributes if present
        if len(self.attributes) > 0:
            attrs = " ".join(
                [
                    f"@{tag}({val})"
                    for tag, val in sorted(self.attributes, key=lambda a: a.name)
                ]
            )
            inv = f"{attrs} {inv}"

        # Add ref cells if present
        if len(self.ref_cells) > 0:
            rcs = ", ".join([f"{n}={arg.doc()}" for (n, arg) in self.ref_cells])
            inv += f"[{rcs}]"

        # Inputs and outputs
        in_defs = ", ".join([f"{p}={a.doc()}" for p, a in self.in_connects])
        out_defs = ", ".join([f"{p}={a.doc()}" for p, a in self.out_connects])
        inv += f"({in_defs})({out_defs})"
        inv += ";"

        return inv

    def with_attr(self, key: str, value: int) -> Invoke:
        self.attributes.add((key, value))
        return self


@dataclass
class While(Control):
    port: Port
    # XXX: This should probably be called the cond_group.
    cond: Optional[CompVar]
    body: Control
    loc: Optional[int] = field(default_factory=PosTable.determine_source_loc)

    def doc(self) -> str:
        cond = ""
        cond += f"while {self.port.doc()}"
        if self.cond:
            cond += f" with {self.cond.doc()}"
        return ctrl_with_pos_attribute(block(cond, self.body.doc(), sep=""), self.loc)


@dataclass
class StaticRepeat(Control):
    num_repeats: int
    body: Control

    def doc(self) -> str:
        cond = f"static repeat {self.num_repeats}"
        return block(cond, self.body.doc(), sep="")


@dataclass
class Empty(Control):
    def doc(self) -> str:
        return ""


@dataclass
class If(Control):
    port: Port
    # XXX: This should probably be called the cond_group.
    cond: Optional[CompVar]
    true_branch: Control
    false_branch: Control = field(default_factory=Empty)
    loc: Optional[int] = field(default_factory=PosTable.determine_source_loc)

    def doc(self) -> str:
        cond = ""
        cond += f"if {self.port.doc()}"
        if self.cond:
            cond += f" with {self.cond.doc()}"
        true_branch = self.true_branch.doc()
        if isinstance(self.false_branch, Empty):
            false_branch = ""
        else:
            false_branch = block(" else", self.false_branch.doc(), sep="")
        return ctrl_with_pos_attribute(
            block(cond, true_branch, sep="") + false_branch, self.loc
        )


@dataclass
class StaticIf(Control):
    port: Port
    true_branch: Control
    false_branch: Control = field(default_factory=Empty)
    loc: Optional[int] = field(default_factory=PosTable.determine_source_loc)

    def doc(self) -> str:
        cond = f"static if {self.port.doc()}"
        true_branch = self.true_branch.doc()
        if isinstance(self.false_branch, Empty):
            false_branch = ""
        else:
            false_branch = block(" else", self.false_branch.doc(), sep="")
        return ctrl_with_pos_attribute(
            block(cond, true_branch, sep="") + false_branch, self.loc
        )


# Standard Library
# XXX: This is a funky way to build the standard library. Maybe we can have a
# better "theory of standard library" to figure out what the right way to do
# this is.
@dataclass
class Stdlib:
    @staticmethod
    def register(bitwidth: int):
        return CompInst("std_reg", [bitwidth])

    @staticmethod
    def wire(bitwidth: int):
        return CompInst("std_wire", [bitwidth])

    @staticmethod
    def constant(bitwidth: int, value: int):
        return CompInst("std_const", [bitwidth, value])

    @staticmethod
    def op(op: str, bitwidth: int, signed: bool):
        return CompInst(f"std_{'s' if signed else ''}{op}", [bitwidth])

    @staticmethod
    def const_mult(bitwidth: int, const: int):
        return CompInst("std_const_mult", [bitwidth, const])

    @staticmethod
    def slice(in_: int, out: int):
        return CompInst("std_slice", [in_, out])

    @staticmethod
    def bit_slice(in_: int, start_idx: int, end_idx: int, out: int):
        return CompInst("std_bit_slice", [in_, start_idx, end_idx, out])

    @staticmethod
    def cat(left_width: int, right_width: int, out_width: int):
        return CompInst("std_cat", [left_width, right_width, out_width])

    @staticmethod
    def pad(in_: int, out: int):
        return CompInst("std_pad", [in_, out])

    @staticmethod
    def comb_mem_d1(bitwidth: int, size: int, idx_size: int):
        return CompInst("comb_mem_d1", [bitwidth, size, idx_size])

    @staticmethod
    def comb_mem_d2(
        bitwidth: int, size0: int, size1: int, idx_size0: int, idx_size1: int
    ):
        return CompInst("comb_mem_d2", [bitwidth, size0, size1, idx_size0, idx_size1])

    @staticmethod
    def comb_mem_d3(
        bitwidth: int,
        size0: int,
        size1: int,
        size2: int,
        idx_size0: int,
        idx_size1: int,
        idx_size2: int,
    ):
        return CompInst(
            "comb_mem_d3",
            [bitwidth, size0, size1, size2, idx_size0, idx_size1, idx_size2],
        )

    @staticmethod
    def comb_mem_d4(
        bitwidth: int,
        size0: int,
        size1: int,
        size2: int,
        size3: int,
        idx_size0: int,
        idx_size1: int,
        idx_size2: int,
        idx_size3: int,
    ):
        return CompInst(
            "comb_mem_d4",
            [
                bitwidth,
                size0,
                size1,
                size2,
                size3,
                idx_size0,
                idx_size1,
                idx_size2,
                idx_size3,
            ],
        )

    @staticmethod
    def seq_mem_d1(bitwidth: int, size: int, idx_size: int):
        return CompInst("seq_mem_d1", [bitwidth, size, idx_size])

    @staticmethod
    def seq_mem_d2(
        bitwidth: int, size0: int, size1: int, idx_size0: int, idx_size1: int
    ):
        return CompInst("seq_mem_d2", [bitwidth, size0, size1, idx_size0, idx_size1])

    @staticmethod
    def seq_mem_d3(
        bitwidth: int,
        size0: int,
        size1: int,
        size2: int,
        idx_size0: int,
        idx_size1: int,
        idx_size2: int,
    ):
        return CompInst(
            "seq_mem_d3",
            [bitwidth, size0, size1, size2, idx_size0, idx_size1, idx_size2],
        )

    @staticmethod
    def seq_mem_d4(
        bitwidth: int,
        size0: int,
        size1: int,
        size2: int,
        size3: int,
        idx_size0: int,
        idx_size1: int,
        idx_size2: int,
        idx_size3: int,
    ):
        return CompInst(
            "seq_mem_d4",
            [
                bitwidth,
                size0,
                size1,
                size2,
                size3,
                idx_size0,
                idx_size1,
                idx_size2,
                idx_size3,
            ],
        )

    # Extended Fixed Point AST
    @staticmethod
    def fixed_point_op(
        op: str, width: int, int_width: int, frac_width: int, signed: bool
    ):
        return CompInst(
            f"std_fp_{'s' if signed else ''}{op}", [width, int_width, frac_width]
        )

    @staticmethod
    def pipelined_mult():
        return CompInst("pipelined_mult", [])

    @staticmethod
    def pipelined_fp_smult(width: int, int_width: int, frac_width: int):
        return CompInst("pipelined_fp_smult", [width, int_width, frac_width])
