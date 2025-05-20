import csv
import os

from classes import (
    TraceData,
    CellMetadata,
    ControlMetadata,
    CycleType,
    CycleTrace,
    StackElementType,
)


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
        "useful-cycles",
        "total-cycles",
        "times-active",
        "avg",
    ] + [
        f"{cat.name} (%)" for cat in tracedata.cycletype_to_cycles
    ]  # fields in CSV file
    stats = []
    totals = {fieldname: 0 for fieldname in fieldnames}
    for cell in tracedata.cell_to_active_cycles:
        component = cell_metadata.cell_to_component[cell]
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
        stats_dict = {
            "cell-name": f"{cell} [{component}]",
            "num-fsms": num_fsms,
            "useful-cycles": len(cell_cat[CycleType.GROUP_OR_PRIMITIVE])
            + len(cell_cat[CycleType.OTHER]),
            "total-cycles": cell_total_cycles,
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
            (len(tracedata.cycletype_to_cycles[cycletype]) / total_cycles) * 100, 1
        )
    totals["avg"] = "-"
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
