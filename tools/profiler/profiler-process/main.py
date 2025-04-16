from datetime import datetime
import os
import json
import sys
import vcdvcd

# import adl_mapping, construct_trace, preprocess
import adl_mapping
import construct_trace
import preprocess
from visuals import flame, tree, timeline, stats


def main(
    vcd_filename,
    cells_json_file,
    tdcc_json_file,
    shared_cells_json,
    adl_mapping_file,
    out_dir,
    flame_out,
):
    print(f"Start time: {datetime.now()}")
    main_shortname, cells_to_components, components_to_cells = (
        preprocess.read_component_cell_names_json(cells_json_file)
    )
    shared_cells_map = preprocess.read_shared_cells_map(shared_cells_json)
    (
        fully_qualified_fsms,
        component_to_num_fsms,
        par_to_children,
        reverse_par_dep_info,
        cell_to_pars,
        par_done_regs,
        cell_to_groups_to_par_parent,
    ) = preprocess.read_tdcc_file(tdcc_json_file, components_to_cells)
    # moving output info out of the converter
    fsm_events = {
        fsm: [{"name": str(0), "cat": "fsm", "ph": "B", "ts": 0}]
        for fsm in fully_qualified_fsms
    }  # won't be fully filled in until create_timeline()
    print(f"Start reading VCD: {datetime.now()}")
    converter = construct_trace.VCDConverter(
        main_shortname,
        cells_to_components,
        fully_qualified_fsms,
        fsm_events,
        set(par_to_children.keys()),
        par_done_regs,
    )
    vcdvcd.VCDVCD(vcd_filename, callbacks=converter)
    signal_prefix = converter.signal_prefix
    main_fullname = converter.main_component
    print(f"Start Postprocessing VCD: {datetime.now()}")

    trace, trace_classified, cell_to_active_cycles = converter.postprocess(
        shared_cells_map
    )  # trace contents: cycle # --> list of stacks, trace_classified is a list: cycle # (indices) --> # useful stacks
    control_groups_trace, control_groups_summary, control_reg_updates, control_reg_updates_per_cycle = (
        converter.postprocess_control()
    )
    cell_to_ordered_pars = construct_trace.order_pars(
        cell_to_pars, par_to_children, reverse_par_dep_info, signal_prefix
    )
    trace_with_pars = construct_trace.add_par_to_trace(
        trace,
        control_groups_trace,
        cell_to_ordered_pars,
        cell_to_groups_to_par_parent,
        main_shortname,
    )
    print(f"End Postprocessing VCD: {datetime.now()}")
    print(f"End reading VCD: {datetime.now()}")
    del converter

    if len(trace) < 100:
        for i in trace_with_pars:
            print(i)
            for stack in trace_with_pars[i]:
                print(f"\t{stack}")

    if not os.path.exists(out_dir):
        os.mkdir(out_dir)
    cats_to_cycles = flame.create_simple_flame_graph(
        trace_classified, control_reg_updates_per_cycle, out_dir
    )
    print(f"End creating simple flame graph: {datetime.now()}")
    stats.write_cell_stats(
        cell_to_active_cycles,
        cats_to_cycles,
        cells_to_components,
        component_to_num_fsms,
        len(trace),
        out_dir,
    )
    stats.write_par_stats(control_groups_summary, cats_to_cycles, trace_with_pars, main_shortname, out_dir)
    print(f"End writing cell stats: {datetime.now()}")
    tree_dict, path_dict = tree.create_tree(trace)
    path_to_edges, all_edges = tree.create_edge_dict(path_dict)

    tree.create_aggregate_tree(trace, out_dir, tree_dict, path_dict)
    tree.create_tree_rankings(
        trace, tree_dict, path_dict, path_to_edges, all_edges, out_dir
    )
    flat_flame_map, scaled_flame_map = flame.create_flame_maps(trace_with_pars)
    flame.write_flame_maps(flat_flame_map, scaled_flame_map, out_dir, flame_out)

    timeline.compute_timeline(
        trace, fsm_events, control_reg_updates, main_fullname, out_dir
    )

    if adl_mapping_file is not None:  # emit ADL flame graphs.
        print("Computing ADL flames...")
        adl_flat_flame, mixed_flat_flame = adl_mapping.convert_flame_map(
            flat_flame_map, adl_mapping_file
        )
        adl_scaled_flame, mixed_scaled_flame = adl_mapping.convert_flame_map(
            scaled_flame_map, adl_mapping_file
        )
        adl_flat_flame_file = os.path.join(out_dir, "adl-flat-flame.folded")
        adl_scaled_flame_file = os.path.join(out_dir, "adl-scaled-flame.folded")
        flame.write_flame_maps(
            adl_flat_flame,
            adl_scaled_flame,
            out_dir,
            adl_flat_flame_file,
            adl_scaled_flame_file,
        )

        mixed_flat_flame_file = os.path.join(out_dir, "mixed-flat-flame.folded")
        mixed_scaled_flame_file = os.path.join(out_dir, "mixed-scaled-flame.folded")
        flame.write_flame_maps(
            mixed_flat_flame,
            mixed_scaled_flame,
            out_dir,
            mixed_flat_flame_file,
            mixed_scaled_flame_file,
        )

    print(f"End time: {datetime.now()}")


if __name__ == "__main__":
    if len(sys.argv) > 6:
        vcd_filename = sys.argv[1]
        cells_json = sys.argv[2]
        fsms_json = sys.argv[3]
        shared_cells_json = sys.argv[4]
        out_dir = sys.argv[5]
        flame_out = sys.argv[6]
        if len(sys.argv) > 7:
            adl_mapping_file = sys.argv[6]
        else:
            adl_mapping_file = None
        print(f"ADL mapping file: {adl_mapping_file}")
        main(
            vcd_filename,
            cells_json,
            fsms_json,
            shared_cells_json,
            adl_mapping_file,
            out_dir,
            flame_out,
        )
    else:
        args_desc = [
            "VCD_FILE",
            "CELLS_JSON",  # FIXME: might want to rename this
            "FSMS_JSON",
            "SHARED_CELLS_JSON",
            "OUT_DIR",
            "FLATTENED_FLAME_OUT",
            "[ADL_MAP_JSON]",
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")
        print("CELLS_JSON: Run the `component_cells` tool")
        print("CELLS_FOR_TIMELINE is an optional ")
        sys.exit(-1)
