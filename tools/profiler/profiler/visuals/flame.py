import os

from profiler.classes.tracedata import (
    PTrace,
    CycleType,
    CycleTrace,
    TraceData,
    ControlRegUpdateType,
    FlameMapMode,
)
from profiler.classes.adl import AdlMap, SourceLoc
from dataclasses import dataclass
from collections import defaultdict

SCALED_FLAME_MULTIPLIER = (
    1000  # [flame graph] multiplier so scaled flame graph will not round up.
)


@dataclass
class FlameTrace:
    flat_flame_map: defaultdict[str, int]
    scaled_flame_map: defaultdict[str, int]

    def __init__(self, string_trace: list[set[str]]):
        self.flat_flame_map = defaultdict(int)
        self.scaled_flame_map = defaultdict(int)
        acc = 0
        for stack_string_set in string_trace:
            # bookkeeping for scaled
            acc += 1
            num_stacks = len(stack_string_set)
            if num_stacks == 0:
                cycle_slice = 1
            else:
                cycle_slice = round(1 / num_stacks, 3)
            # the last slice is adjusted s.t. all of the slices together add up to 1.
            last_cycle_slice = 1 - (cycle_slice * (num_stacks - 1))
            acc = 0
            for stack_id in stack_string_set:
                self.flat_flame_map[stack_id] += 1
                # scaled flame
                slice_to_add = cycle_slice if acc < num_stacks - 1 else last_cycle_slice
                self.scaled_flame_map[stack_id] += (
                    slice_to_add * SCALED_FLAME_MULTIPLIER
                )
                acc += 1

    def write_flame_maps(
        self,
        flames_out_dir: str,
        flame_out_file: str,
        scaled_flame_out_file: str = None,
    ):
        """
        Utility function for writing flat and scaled flame maps to file.
        flame_out_file and scaled_flame_out_filename are full paths.
        """
        if not os.path.exists(flames_out_dir):
            os.mkdir(flames_out_dir)

        # write flat flame map
        self.write_flame_map(self.flat_flame_map, flame_out_file)

        # write scaled flame map
        if scaled_flame_out_file is None:
            scaled_flame_out_file = os.path.join(flames_out_dir, "scaled-flame.folded")
        self.write_flame_map(self.scaled_flame_map, scaled_flame_out_file)

    def write_flame_map(self, flame_map: dict[str, int], flame_out_file: str):
        """
        Utility function for outputting a flame graph to file.
        """
        with open(flame_out_file, "w") as flame_out:
            for stack in flame_map:
                flame_out.write(f"{stack} {flame_map[stack]}\n")


def create_and_write_calyx_flame_maps(
    trace: PTrace, out_dir: str, flame_out: str, mode: FlameMapMode = FlameMapMode.CALYX
) -> tuple[dict[str, int], dict[str, int]]:
    """
    Function to create flame maps for Calyx-style traces.
    """
    # create string version
    string_trace: list[set[str]] = trace.string_repr(mode)
    flametrace: FlameTrace = FlameTrace(string_trace)
    flametrace.write_flame_maps(out_dir, flame_out)


def create_and_write_dahlia_flame_maps(
    tracedata: TraceData, adl_mapping_file: str, out_dir: str
):
    calyx_trace: PTrace = tracedata.trace_with_control_groups
    adl_map = AdlMap(adl_mapping_file)
    dahlia_string_trace: list[set[str]] = []
    # create string version with Dahlia constructs
    for i in calyx_trace:
        i_string_set = set()
        # find leaf groups (there could be some in parallel)
        leaf_groups: set = calyx_trace[i].find_leaf_groups()
        group_map = adl_map.group_map.get("main")
        for group in leaf_groups:
            if group not in group_map:
                entry = f"CALYX: '{group}'"
            else:
                group_sourceloc: SourceLoc = group_map[group]
                entry = group_sourceloc.adl_str()
            i_string_set.add(entry)
        dahlia_string_trace.append(i_string_set)

    flametrace: FlameTrace = FlameTrace(dahlia_string_trace)
    adl_flat_flame_file = os.path.join(out_dir, "adl-flat-flame.folded")
    adl_scaled_flame_file = os.path.join(out_dir, "adl-scaled-flame.folded")
    flametrace.write_flame_maps(
        out_dir, adl_flat_flame_file, scaled_flame_out_file=adl_scaled_flame_file
    )


def create_flame_maps(
    trace: PTrace, mode: FlameMapMode = FlameMapMode.CALYX
) -> tuple[dict[str, int], dict[str, int]]:
    """
    Creates flat and scaled flame maps from a trace.
    """

    # flat flame graph; each par arm is counted for 1 cycle
    flat_flame_map = {}  # stack to number of cycles
    for i in trace:
        i_trace: CycleTrace = trace[i]
        for stack_id in i_trace.get_stack_str_list(mode):
            if stack_id not in flat_flame_map:
                flat_flame_map[stack_id] = 1
            else:
                flat_flame_map[stack_id] += 1

    # scaled flame graph; each cycle is divided by the number of par arms that are concurrently active.
    scaled_flame_map = {}
    for i in trace:
        i_trace = trace[i]
        num_stacks = i_trace.get_num_stacks()
        cycle_slice = round(1 / num_stacks, 3)
        last_cycle_slice = 1 - (cycle_slice * (num_stacks - 1))
        acc = 0
        for stack_id in i_trace.get_stack_str_list(mode):
            slice_to_add = cycle_slice if acc < num_stacks - 1 else last_cycle_slice
            if stack_id not in scaled_flame_map:
                scaled_flame_map[stack_id] = slice_to_add * SCALED_FLAME_MULTIPLIER
            else:
                scaled_flame_map[stack_id] += slice_to_add * SCALED_FLAME_MULTIPLIER
            acc += 1

    return flat_flame_map, scaled_flame_map


def create_simple_flame_graph(
    tracedata: TraceData, control_reg_updates: dict[int, ControlRegUpdateType]
):
    """
    Create and output a very simple overview flame graph that attributes cycles to categories
    describing how "useful" a cycle is.
    """
    flame_base_map: dict[CycleType, set[int]] = {t: set() for t in CycleType}
    for i in tracedata.trace:
        cycle_trace = tracedata.trace[i]
        if cycle_trace.is_useful_cycle:
            cycle_type = CycleType.GROUP_OR_PRIMITIVE
        elif i not in control_reg_updates:
            # most likely cycles devoted to compiler-generated groups (repeats, etc)
            cycle_type = CycleType.OTHER
            cycle_trace.is_useful_cycle = (
                True  # FIXME: hack to flag this as a "useful" cycle
            )
        else:
            match control_reg_updates[i]:
                case ControlRegUpdateType.FSM:
                    cycle_type = CycleType.FSM_UPDATE
                case ControlRegUpdateType.PAR_DONE:
                    cycle_type = CycleType.PD_UPDATE
                case ControlRegUpdateType.BOTH:
                    cycle_type = CycleType.MULT_CONTROL
        flame_base_map[cycle_type].add(i)

    # modify names to contain their cycles (for easier viewing)
    # flame_map = {}
    # for key in flame_base_map:
    #     cycles = len(flame_base_map[key])
    #     flame_map[f"{key.name} ({cycles})"] = cycles
    # write_flame_map(flame_map, os.path.join(out_dir, "overview.folded"))
    tracedata.cycletype_to_cycles = flame_base_map
