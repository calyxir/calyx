import csv
import os

from profiler.classes import (
    TraceData,
    CellMetadata,
    ControlMetadata,
    CycleType,
    CycleTrace,
    StackElementType,
    GroupSummary,
)


def create_group_summaries(cell_metadata: CellMetadata, tracedata: TraceData):
    """
    Processes the trace to identify group activation blocks and collect group statistics.
    """
    group_summaries: dict[str, GroupSummary] = {}  # names would be component.group
    currently_active_to_start: dict[str, int] = {}
    for i in tracedata.trace:
        active_this_cycle: set[str] = set()
        for stack in tracedata.trace[i].stacks:
            stack_acc = cell_metadata.main_component
            current_component = cell_metadata.main_component
            for stack_elem in stack:
                match stack_elem.element_type:
                    case StackElementType.CELL:
                        if not stack_elem.is_main:
                            stack_acc = f"{stack_acc}.{stack_elem.name}"
                            current_component = cell_metadata.get_component_of_cell(
                                stack_acc
                            )
                    case StackElementType.GROUP:
                        group_id = f"{current_component}.{stack_elem.name}"
                        if group_id not in group_summaries:
                            group_summaries[group_id] = GroupSummary(group_id)
                        active_this_cycle.add(group_id)

        # groups that ended this cycle
        for done_group in set(currently_active_to_start.keys()).difference(
            active_this_cycle
        ):
            start_cycle = currently_active_to_start[done_group]
            group_summaries[done_group].register_interval(range(start_cycle, i))
            del currently_active_to_start[done_group]
        # groups that started this cycle
        for new_group in active_this_cycle.difference(
            set(currently_active_to_start.keys())
        ):
            currently_active_to_start[new_group] = i

    # groups that are active until the end
    for still_active_group in currently_active_to_start:
        start_cycle = currently_active_to_start[still_active_group]
        group_summaries[still_active_group].register_interval(
            range(start_cycle, len(tracedata.trace))
        )

    return group_summaries


def write_group_stats(cell_metadata: CellMetadata, tracedata: TraceData, out_dir: str):
    """
    Collects and writes statistics information about groups to group-stats.csv.
    """
    group_summaries = create_group_summaries(cell_metadata, tracedata)
    fieldnames = GroupSummary.fieldnames()
    stats_list = []
    for group in sorted(group_summaries.keys()):
        stats_list.append(group_summaries[group].stats())
    with open(
        os.path.join(out_dir, "group-stats.csv"), "w", encoding="utf-8"
    ) as csvFile:
        writer = csv.DictWriter(
            csvFile, fieldnames=fieldnames, lineterminator=os.linesep
        )
        writer.writeheader()
        writer.writerows(stats_list)


def write_cell_stats(
    cell_metadata: CellMetadata,
    control_metadata: ControlMetadata,
    tracedata: TraceData,
    out_dir: str,
):
    """
    Collect and write statistics information about cells to cell-stats.csv.
    """
    fieldnames = [
        "cell-name",
        "num-fsms",
        "total-cycles",
        "times-active",
        "avg",
        "useful-cycles",
        "useful-cycles (%)",
    ] + [
        f"{cat.name} (%)" for cat in tracedata.cycletype_to_cycles
    ]  # fields in CSV file
    stats = []
    totals = {fieldname: 0 for fieldname in fieldnames}
    for cell in tracedata.cell_to_active_cycles:
        component = cell_metadata.get_component_of_cell(cell)
        if component in control_metadata.component_to_fsms:
            num_fsms = len(control_metadata.component_to_fsms[component])
        else:
            num_fsms = 0
        cell_total_cycles = len(tracedata.cell_to_active_cycles[cell].active_cycles)
        times_active = tracedata.cell_to_active_cycles[cell].num_times_active
        cell_cat = {}
        for cycletype in tracedata.cycletype_to_cycles:
            cell_cat[cycletype] = tracedata.cell_to_active_cycles[
                cell
            ].active_cycles.intersection(tracedata.cycletype_to_cycles[cycletype])
        avg_cycles = round(cell_total_cycles / times_active, 2)
        useful_cycles = len(cell_cat[CycleType.GROUP_OR_PRIMITIVE]) + len(
            cell_cat[CycleType.OTHER]
        )
        stats_dict = {
            "cell-name": f"{cell} [{component}]",
            "num-fsms": num_fsms,
            "useful-cycles": useful_cycles,
            "total-cycles": cell_total_cycles,
            "useful-cycles (%)": round((useful_cycles / cell_total_cycles) * 100, 1),
            "times-active": times_active,
            "avg": avg_cycles,
        }
        # aggregate stats that should be summed over
        totals["num-fsms"] += num_fsms
        for cycletype in tracedata.cycletype_to_cycles:
            stats_dict[f"{cycletype.name} (%)"] = round(
                (len(cell_cat[cycletype]) / cell_total_cycles) * 100, 1
            )
        stats.append(stats_dict)
    # total: aggregate other stats that shouldn't just be summed over
    totals["cell-name"] = "TOTAL"
    total_cycles = len(tracedata.trace)
    totals["total-cycles"] = total_cycles
    for cycletype in tracedata.cycletype_to_cycles:
        match cycletype:
            case CycleType.GROUP_OR_PRIMITIVE | CycleType.OTHER:
                totals["useful-cycles"] += len(tracedata.cycletype_to_cycles[cycletype])
        totals[f"{cycletype.name} (%)"] = round(
            (len(tracedata.cycletype_to_cycles[cycletype]) / total_cycles) * 100, 2
        )
    totals["times-active"] = "-"
    totals["avg"] = "-"
    totals["useful-cycles (%)"] = round(
        (totals["useful-cycles"] / total_cycles) * 100, 1
    )
    stats.sort(key=lambda e: e["total-cycles"], reverse=True)
    stats.append(totals)  # total should come at the end
    with open(
        os.path.join(out_dir, "cell-stats.csv"), "w", encoding="utf-8"
    ) as csvFile:
        writer = csv.DictWriter(
            csvFile, fieldnames=fieldnames, lineterminator=os.linesep
        )
        writer.writeheader()
        writer.writerows(stats)


def compute_par_useful_work(
    fully_qualified_par_name,
    active_cycles: set[int],
    trace: dict[int, CycleTrace],
):
    """
    Utility function to compute the amount of "flattened" work we get out of a par.
    """
    # super hacky way to get number of flattened useful cycles we obtained
    acc = 0
    # FIXME: this may not work for nested pars. Should explicitly test
    par_cell_name = fully_qualified_par_name.split(".")[-2]
    par_name = fully_qualified_par_name.split(".")[-1]
    for cycle in active_cycles:  # cycles where the par group is active
        for stack in trace[cycle].stacks:
            in_par_cell = False  # are we in the cell that the par is active in?
            in_par = False  # are we in the par itself?
            for stack_elem in stack:
                match stack_elem.element_type:
                    case StackElementType.CELL:
                        if in_par_cell:
                            # we were previously in the cell that the par lived in but no longer are.
                            break
                        elif stack_elem.name == par_cell_name:
                            in_par_cell = True
                    case StackElementType.CONTROL_GROUP:
                        if in_par_cell and stack_elem.name == par_name:
                            in_par = True
                    case StackElementType.GROUP:
                        if in_par:
                            acc += 1
                    # ignoring primitives for now as they can't happen without a group.

    return acc


def write_par_stats(tracedata: TraceData, out_dir):
    """
    Collect and output statistics about TDCC-defined par groups to ctrl-group-stats.csv.
    """
    # exit early if there are no control groups to check
    if len(tracedata.control_group_to_active_cycles) == 0:
        print(
            "[write_par_stats] No par/control groups to emit information about! Skipping..."
        )
        return
    fieldnames = [
        "group-name",
        "flattened-cycles",
        "useful-cycles",
        "total-cycles",
        "flattened-cycles (%)",
        "useful-cycles (%)",
        "times-active",
    ]
    stats = []
    totals = {fieldname: 0 for fieldname in fieldnames}
    for group in tracedata.control_group_to_active_cycles:
        flattened_useful_cycles = compute_par_useful_work(
            group,
            tracedata.control_group_to_active_cycles[group].active_cycles,
            tracedata.trace_with_control_groups,
        )
        active_cycles_set = tracedata.control_group_to_active_cycles[
            group
        ].active_cycles
        num_active_cycles = len(active_cycles_set)
        useful_cycles = len(
            active_cycles_set.intersection(
                tracedata.cycletype_to_cycles[CycleType.GROUP_OR_PRIMITIVE]
            )
        )
        flattened_cycle_percent = round(
            (flattened_useful_cycles / num_active_cycles) * 100, 1
        )
        useful_cycle_percent = round((useful_cycles / num_active_cycles) * 100, 1)
        stats_dict = {
            "group-name": group,
            "flattened-cycles": flattened_useful_cycles,
            "useful-cycles": useful_cycles,
            "total-cycles": num_active_cycles,
            "flattened-cycles (%)": flattened_cycle_percent,
            "useful-cycles (%)": useful_cycle_percent,
            "times-active": tracedata.control_group_to_active_cycles[
                group
            ].num_times_active,
        }
        for field in stats_dict:
            if field not in ["group-name", "useful-cycles (%)", "flattened-cycles (%)"]:
                totals[field] += stats_dict[field]
        stats.append(stats_dict)
    totals["group-name"] = "TOTAL"
    totals["flattened-cycles (%)"] = round(
        (totals["flattened-cycles"] / totals["total-cycles"]) * 100, 1
    )
    totals["useful-cycles (%)"] = round(
        (totals["useful-cycles"] / totals["total-cycles"]) * 100, 1
    )
    stats.append(totals)

    with open(
        os.path.join(out_dir, "ctrl-group-stats.csv"), "w", encoding="utf-8"
    ) as csvFile:
        writer = csv.DictWriter(
            csvFile, fieldnames=fieldnames, lineterminator=os.linesep
        )
        writer.writeheader()
        writer.writerows(stats)
