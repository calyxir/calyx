import os

SCALED_FLAME_MULTIPLIER = (
    1000  # [flame graph] multiplier so scaled flame graph will not round up.
)


"""
Utility function for outputting a flame graph to file.
"""


def write_flame_map(flame_map, flame_out_file):
    with open(flame_out_file, "w") as flame_out:
        for stack in flame_map:
            flame_out.write(f"{stack} {flame_map[stack]}\n")


"""
Utility function for writing flat and scaled flame maps to file.
"""


def write_flame_maps(
    flat_flame_map,
    scaled_flame_map,
    flames_out_dir,
    flame_out_file,
    scaled_flame_out_file=None,
):
    if not os.path.exists(flames_out_dir):
        os.mkdir(flames_out_dir)

    # write flat flame map
    write_flame_map(flat_flame_map, flame_out_file)

    # write scaled flame map
    if scaled_flame_out_file is None:
        scaled_flame_out_file = os.path.join(flames_out_dir, "scaled-flame.folded")
    write_flame_map(scaled_flame_map, scaled_flame_out_file)


"""
Creates flat and scaled flame maps from a trace.
"""


def create_flame_maps(trace):
    # flat flame graph; each par arm is counted for 1 cycle
    flat_flame_map = {}  # stack to number of cycles
    for i in trace:
        for stack_list in trace[i]:
            stack_id = ";".join(stack_list)
            if stack_id not in flat_flame_map:
                flat_flame_map[stack_id] = 1
            else:
                flat_flame_map[stack_id] += 1

    # scaled flame graph; each cycle is divided by the number of par arms that are concurrently active.
    scaled_flame_map = {}
    for i in trace:
        num_stacks = len(trace[i])
        cycle_slice = round(1 / num_stacks, 3)
        last_cycle_slice = 1 - (cycle_slice * (num_stacks - 1))
        acc = 0
        for stack_list in trace[i]:
            stack_id = ";".join(stack_list)
            slice_to_add = cycle_slice if acc < num_stacks - 1 else last_cycle_slice
            if stack_id not in scaled_flame_map:
                scaled_flame_map[stack_id] = slice_to_add * SCALED_FLAME_MULTIPLIER
            else:
                scaled_flame_map[stack_id] += slice_to_add * SCALED_FLAME_MULTIPLIER
            acc += 1

    return flat_flame_map, scaled_flame_map


"""
Create and output a very simple overview flame graph that attributes cycles to categories
describing how "useful" a cycle is.
"""


def create_simple_flame_graph(classified_trace, control_reg_updates, out_dir):
    flame_base_map = {
        "group/primitive": [],  # at least one group/primitive is executing this cycle
        "fsm": [],  # only fsm updates are happening this cycle
        "par-done": [],  # only pd register updates are happening this cycle
        "mult-ctrl": [],  # fsm and par-done
        "other": [],
    }
    for i in range(len(classified_trace)):
        if classified_trace[i] > 0:
            flame_base_map["group/primitive"].append(i)
        elif (
            i not in control_reg_updates
        ):  # I suspect this is 1 cycle to execute a combinational group.
            flame_base_map["other"].append(i)
            classified_trace[i] = 1  # FIXME: hack to flag this as a "useful" cycle
        elif control_reg_updates[i] == "both":
            flame_base_map["mult-ctrl"].append(i)
        else:
            flame_base_map[control_reg_updates[i]].append(i)
    # modify names to contain their cycles (for easier viewing)
    flame_map = {key: len(flame_base_map[key]) for key in flame_base_map}
    for label in list(flame_map.keys()):
        flame_map[f"{label} ({flame_map[label]})"] = flame_map[label]
        del flame_map[label]
    write_flame_map(flame_map, os.path.join(out_dir, "overview.folded"))
    return flame_base_map
