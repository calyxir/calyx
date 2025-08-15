import os

from profiler.classes import (
    CycleTrace,
    CycleType,
    TraceData,
    ControlRegUpdateType,
    FlameMapMode,
)

SCALED_FLAME_MULTIPLIER = (
    1000  # [flame graph] multiplier so scaled flame graph will not round up.
)


def write_flame_map(flame_map: dict[str, int], flame_out_file: str):
    """
    Utility function for outputting a flame graph to file.
    """
    with open(flame_out_file, "w") as flame_out:
        for stack in flame_map:
            flame_out.write(f"{stack} {flame_map[stack]}\n")


def write_flame_maps(
    flat_flame_map,
    scaled_flame_map,
    flames_out_dir,
    flame_out_file: str,
    scaled_flame_out_filename:str =None,
):
    """
    Utility function for writing flat and scaled flame maps to file.
    flame_out_file is the full path; scaled_flame_out_filename is just the name of the file.
    FIXME: we should be consistent with the paths.
    """
    if not os.path.exists(flames_out_dir):
        os.mkdir(flames_out_dir)

    # write flat flame map
    write_flame_map(flat_flame_map, flame_out_file)

    # write scaled flame map
    if scaled_flame_out_filename is None:
        scaled_flame_out_filename = "scaled-flame.folded"
    scaled_flame_out_file = os.path.join(flames_out_dir, scaled_flame_out_filename)
    write_flame_map(scaled_flame_map, scaled_flame_out_file)


def create_flame_maps(
    trace: dict[int, CycleTrace], mode: FlameMapMode = FlameMapMode.CALYX
) -> tuple[dict[str, int], dict[str, int]]:
    """
    Creates flat and scaled flame maps from a trace.
    """

    # flat flame graph; each par arm is counted for 1 cycle
    flat_flame_map = {}  # stack to number of cycles
    for i in trace:
        for stack_id in trace[i].get_stack_str_list(mode):
            if stack_id not in flat_flame_map:
                flat_flame_map[stack_id] = 1
            else:
                flat_flame_map[stack_id] += 1

    # scaled flame graph; each cycle is divided by the number of par arms that are concurrently active.
    scaled_flame_map = {}
    for i in trace:
        num_stacks = trace[i].get_num_stacks()
        cycle_slice = round(1 / num_stacks, 3)
        last_cycle_slice = 1 - (cycle_slice * (num_stacks - 1))
        acc = 0
        for stack_id in trace[i].get_stack_str_list(mode):
            slice_to_add = cycle_slice if acc < num_stacks - 1 else last_cycle_slice
            if stack_id not in scaled_flame_map:
                scaled_flame_map[stack_id] = slice_to_add * SCALED_FLAME_MULTIPLIER
            else:
                scaled_flame_map[stack_id] += slice_to_add * SCALED_FLAME_MULTIPLIER
            acc += 1

    return flat_flame_map, scaled_flame_map


def create_simple_flame_graph(
    tracedata: TraceData, control_reg_updates: dict[int, ControlRegUpdateType], out_dir
):
    """
    Create and output a very simple overview flame graph that attributes cycles to categories
    describing how "useful" a cycle is.
    """
    flame_base_map: dict[CycleType, set[int]] = {t: set() for t in CycleType}
    for i in range(len(tracedata.trace)):
        if tracedata.trace[i].is_useful_cycle:
            cycle_type = CycleType.GROUP_OR_PRIMITIVE
        elif i not in control_reg_updates:
            # most likely cycles devoted to compiler-generated groups (repeats, etc)
            cycle_type = CycleType.OTHER
            tracedata.trace[
                i
            ].is_useful_cycle = True  # FIXME: hack to flag this as a "useful" cycle
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
    flame_map = {}
    for key in flame_base_map:
        cycles = len(flame_base_map[key])
        flame_map[f"{key.name} ({cycles})"] = cycles
    write_flame_map(flame_map, os.path.join(out_dir, "overview.folded"))
    tracedata.cycletype_to_cycles = flame_base_map
