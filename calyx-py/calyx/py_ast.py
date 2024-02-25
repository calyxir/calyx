from __future__ import annotations  # Used for circular dependencies.
from dataclasses import dataclass, field
from typing import List, Any, Tuple, Optional
from calyx.utils import block


@dataclass
class Emittable:
    def doc(self) -> str:
        assert False, f"`doc` not implemented for {type(self).__name__}"

    def emit(self):
        print(self.doc())


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
        return out


# Component
@dataclass
class Component:
    name: str
    attributes : list[Attribute]
    inputs: list[PortDef]
    outputs: list[PortDef]
    wires: list[Structure]
    cells: list[Cell]
    controls: Control
    latency: Optional[int]

    def __init__(
        self,
        name: str,
        inputs: list[PortDef],
        outputs: list[PortDef],
        structs: list[Structure],
        controls: Control,
        attributes: Optional[list[CompAttribute]] = None,
        latency: Optional[int] = None,
    ):
        self.name = name
        self.attributes = attributes
        self.inputs = inputs
        self.outputs = outputs
        self.controls = controls
        self.latency = latency

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
        latency_annotation = (
            f"static<{self.latency}> " if self.latency is not None else ""
        )
        attribute_annotation = f"<{', '.join([f'{a.doc()}' for a in self.attributes])}>" if self.attributes else ""
        signature = f"{latency_annotation}component {self.name}{attribute_annotation}({ins}) -> ({outs})"
        cells = block("cells", [c.doc() for c in self.cells])
        wires = block("wires", [w.doc() for w in self.wires])
        controls = block("control", [self.controls.doc()])
        return block(signature, [cells, wires, controls])


#Attribute
@dataclass
class Attribute(Emittable):
    pass

@dataclass
class CompAttribute(Attribute):
    name: str
    value: int

    def doc(self) -> str:
        return f"\"{self.name}\"={self.value}"


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


@dataclass
class ThisPort(Port):
    id: CompVar

    def doc(self) -> str:
        return self.id.doc()


@dataclass
class HolePort(Port):
    id: CompVar
    name: str

    def doc(self) -> str:
        return f"{self.id.doc()}[{self.name}]"


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


@dataclass
class PortDef(Emittable):
    id: CompVar
    width: int

    def doc(self) -> str:
        return f"{self.id.doc()}: {self.width}"


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

    def doc(self) -> str:
        assert not (
            self.is_ref and self.is_external
        ), "Cell cannot be both a ref and external"
        external = "@external " if self.is_external else ""
        ref = "ref " if self.is_ref else ""
        return f"{external}{ref}{self.id.doc()} = {self.comp.doc()};"


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

    def doc(self) -> str:
        static_delay_attr = (
            "" if self.static_delay is None else f'<"promotable"={self.static_delay}>'
        )
        return block(
            f"group {self.id.doc()}{static_delay_attr}",
            [c.doc() for c in self.connections],
        )


@dataclass
class CombGroup(Structure):
    id: CompVar
    connections: list[Connect]

    def doc(self) -> str:
        return block(
            f"comb group {self.id.doc()}",
            [c.doc() for c in self.connections],
        )


@dataclass
class StaticGroup(Structure):
    id: CompVar
    connections: list[Connect]
    latency: int

    def doc(self) -> str:
        return block(
            f"static<{self.latency}> group {self.id.doc()}",
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


# Control
@dataclass
class Control(Emittable):
    pass


@dataclass
class Enable(Control):
    stmt: str

    def doc(self) -> str:
        return f"{self.stmt};"


@dataclass
class SeqComp(Control):
    stmts: list[Control]

    def doc(self) -> str:
        return block("seq", [s.doc() for s in self.stmts])


@dataclass
class StaticSeqComp(Control):
    stmts: list[Control]

    def doc(self) -> str:
        return block("static seq", [s.doc() for s in self.stmts])


@dataclass
class ParComp(Control):
    stmts: list[Control]

    def doc(self) -> str:
        return block("par", [s.doc() for s in self.stmts])


@dataclass
class StaticParComp(Control):
    stmts: list[Control]

    def doc(self) -> str:
        return block("static par", [s.doc() for s in self.stmts])


@dataclass
class Invoke(Control):
    id: CompVar
    in_connects: List[Tuple[str, Port]]
    out_connects: List[Tuple[str, Port]]
    ref_cells: List[Tuple[str, CompVar]] = field(default_factory=list)
    comb_group: Optional[CompVar] = None
    attributes: List[Tuple[str, int]] = field(default_factory=list)

    def doc(self) -> str:
        inv = f"invoke {self.id.doc()}"

        # Add attributes if present
        if len(self.attributes) > 0:
            attrs = " ".join([f"@{tag}({val})" for tag, val in self.attributes])
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

        return inv

    def with_attr(self, key: str, value: int) -> Invoke:
        self.attributes.append((key, value))
        return self


@dataclass
class StaticInvoke(Control):
    id: CompVar
    in_connects: List[Tuple[str, Port]]
    out_connects: List[Tuple[str, Port]]
    ref_cells: List[Tuple[str, CompVar]] = field(default_factory=list)
    attributes: List[Tuple[str, int]] = field(default_factory=list)

    def doc(self) -> str:
        inv = f"static invoke {self.id.doc()}"

        # Add attributes if present
        if len(self.attributes) > 0:
            attrs = " ".join([f"@{tag}({val})" for tag, val in self.attributes])
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
        self.attributes.append((key, value))
        return self


@dataclass
class While(Control):
    port: Port
    # XXX: This should probably be called the cond_group.
    cond: Optional[CompVar]
    body: Control

    def doc(self) -> str:
        cond = f"while {self.port.doc()}"
        if self.cond:
            cond += f" with {self.cond.doc()}"
        return block(cond, self.body.doc(), sep="")


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

    def doc(self) -> str:
        cond = f"if {self.port.doc()}"
        if self.cond:
            cond += f" with {self.cond.doc()}"
        true_branch = self.true_branch.doc()
        if isinstance(self.false_branch, Empty):
            false_branch = ""
        else:
            false_branch = block(" else", self.false_branch.doc(), sep="")
        return block(cond, true_branch, sep="") + false_branch


@dataclass
class StaticIf(Control):
    port: Port
    true_branch: Control
    false_branch: Control = field(default_factory=Empty)

    def doc(self) -> str:
        cond = f"static if {self.port.doc()}"
        true_branch = self.true_branch.doc()
        if isinstance(self.false_branch, Empty):
            false_branch = ""
        else:
            false_branch = block(" else", self.false_branch.doc(), sep="")
        return block(cond, true_branch, sep="") + false_branch


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
        return CompInst(f'std_{"s" if signed else ""}{op}', [bitwidth])

    @staticmethod
    def slice(in_: int, out: int):
        return CompInst("std_slice", [in_, out])

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
            f'std_fp_{"s" if signed else ""}{op}', [width, int_width, frac_width]
        )

    @staticmethod
    def pipelined_mult():
        return CompInst(f"pipelined_mult", [])

    @staticmethod
    def pipelined_fp_smult(width: int, int_width: int, frac_width: int):
        return CompInst(f"pipelined_fp_smult", [width, int_width, frac_width])
