import csv
import os

from classes import TraceData, CellMetadata, ControlMetadata, CycleType

def write_cell_stats(
    cell_metadata: CellMetadata,
    control_metadata: ControlMetadata,
    tracedata: TraceData,
    # cell_to_active_cycles,
    # cats_to_cycles,
    # cells_to_components,
    # component_to_num_fsms,
    # total_cycles,
    out_dir: str
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
    ] + [f"{cat.name} (%)" for cat in tracedata.cycletype_to_cycles]  # fields in CSV file
    stats = []
    totals = {fieldname: 0 for fieldname in fieldnames}
    for cell in tracedata.cell_to_active_cycles:
        component = cell_metadata.cell_to_component[cell]
        num_fsms = len(control_metadata.component_to_fsms[component])
        cell_total_cycles = len(tracedata.cell_to_active_cycles[cell].active_cycles)
        times_active = tracedata.cell_to_active_cycles[cell].num_times_active
        cell_cat = {}
        for cat in tracedata.cycletype_to_cycles:
            cell_cat[cat] = (
                tracedata.cell_to_active_cycles[cell].active_cycles.intersection(
                    tracedata.cycletype_to_cycles[cat]
                )
            )
        avg_cycles = round(cell_total_cycles / times_active, 2)
        stats_dict = {
            "cell-name": f"{cell} [{component}]",
            "num-fsms": num_fsms,
            "useful-cycles": len(cell_cat[CycleType.GROUP_OR_PRIMITIVE]) + len(cell_cat[CycleType.OTHER]),
            "total-cycles": cell_total_cycles,
            "times-active": times_active,
            "avg": avg_cycles,
        }
        # aggregate stats that should be summed over
        totals["num-fsms"] += num_fsms
        for cycletype in tracedata.cycletype_to_cycles:
            stats_dict[f"{cycletype.name} (%)"] = round(
                (len(cell_cat[cat]) / cell_total_cycles) * 100, 1
            )
        stats.append(stats_dict)
    # total: aggregate other stats that shouldn't just be summed over
    totals["cell-name"] = "TOTAL"
    total_cycles = len(tracedata.trace)
    totals["total-cycles"] = total_cycles
    totals["useful-cycles"] = len(tracedata.cycletype_to_cycles[CycleType.GROUP_OR_PRIMITIVE]) + len(tracedata.cycletype_to_cycles[CycleType.OTHER])
    # totals["useful-cycles"] = len(filter((lambda t: t.is_useful_cycle), tracedata.trace.values))
    for cycletype in tracedata.cycletype_to_cycles:
        match cycletype:
            case CycleType.GROUP_OR_PRIMITIVE | CycleType.OTHER:
                totals["useful-cycles"] += len(tracedata.cycletype_to_cycles[cycletype])
        totals[f"{cycletype.name} (%)"] = round((len(tracedata.cycletype_to_cycles[cycletype]) / total_cycles) * 100, 1)
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
    fully_qualified_par_name, active_cycles, trace, main_shortname
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
        for stack in trace[cycle]:
            in_par_cell = False  # are we in the cell that the par is active in?
            in_par = False  # are we in the par itself?
            for stack_elem in stack:
                if stack_elem == main_shortname or "[" in stack_elem:
                    # in a cell
                    if in_par_cell:  # we were previously in the cell that the par lived in but no longer are.
                        break
                    elif stack_elem.split("[")[0] == par_cell_name:
                        in_par_cell = True
                elif in_par_cell and stack_elem == f"{par_name} (ctrl)":
                    in_par = True
                elif (
                    in_par and "(" not in stack_elem
                ):  # let's ignore primitives as they can't happen without a group?
                    # encountered a group
                    acc += 1

    return acc



def write_par_stats(
    control_groups_summary, cats_to_cycles, trace_with_ctrl, main_shortname, out_dir
):
    """
    Collect and output statistics about TDCC-defined par groups to ctrl-group-stats.csv.
    """    
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
    for group in control_groups_summary:
        flattened_useful_cycles = compute_par_useful_work(
            group,
            control_groups_summary[group]["active-cycles"],
            trace_with_ctrl,
            main_shortname,
        )
        active_cycles_set = set(control_groups_summary[group]["active-cycles"])
        num_active_cycles = len(active_cycles_set)
        useful_cycles = len(
            active_cycles_set.intersection(cats_to_cycles["group/primitive"])
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
            "times-active": control_groups_summary[group]["num-times-active"],
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
