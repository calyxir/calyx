import csv
import os


def write_cell_stats(
    cell_to_active_cycles,
    cats_to_cycles,
    cells_to_components,
    component_to_num_fsms,
    total_cycles,
    out_dir,
):
    fieldnames = [
        "cell-name",
        "num-fsms",
        "useful-cycles",
        "total-cycles",
        "times-active",
        "avg",
    ] + [f"{cat} (%)" for cat in cats_to_cycles]  # fields in CSV file
    stats = []
    totals = {fieldname: 0 for fieldname in fieldnames}
    for cell in cell_to_active_cycles:
        component = cells_to_components[cell]
        num_fsms = component_to_num_fsms[component]
        cell_total_cycles = len(cell_to_active_cycles[cell]["active-cycles"])
        times_active = cell_to_active_cycles[cell]["num-times-active"]
        cell_cat = {cat: set() for cat in cats_to_cycles}
        for cat in cats_to_cycles:
            cell_cat[cat].update(
                cell_to_active_cycles[cell]["active-cycles"].intersection(
                    cats_to_cycles[cat]
                )
            )

        avg_cycles = round(cell_total_cycles / times_active, 2)
        stats_dict = {
            "cell-name": f"{cell} [{component}]",
            "num-fsms": num_fsms,
            "useful-cycles": len(cell_cat["group/primitive"]) + len(cell_cat["other"]),
            "total-cycles": cell_total_cycles,
            "times-active": times_active,
            "avg": avg_cycles,
        }
        # aggregate stats that should be summed over
        totals["num-fsms"] += num_fsms
        for cat in cats_to_cycles:
            stats_dict[f"{cat} (%)"] = round(
                (len(cell_cat[cat]) / cell_total_cycles) * 100, 1
            )
        stats.append(stats_dict)
    # total: aggregate other stats that shouldn't just be summed over
    totals["cell-name"] = "TOTAL"
    totals["total-cycles"] = total_cycles
    for cat in cats_to_cycles:
        if cat == "group/primitive" or cat == "other":
            totals["useful-cycles"] += len(cats_to_cycles[cat])
        totals[f"{cat} (%)"] = round((len(cats_to_cycles[cat]) / total_cycles) * 100, 1)
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
