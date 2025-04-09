import json

"""
Generates a list of all of the components to potential cell names
`prefix` is the cell's "path" (ex. for a cell "my_cell" defined in "main", the prefix would be "TOP.toplevel.main")
The initial value of curr_component should be the top level/main component
"""


def build_components_to_cells(
    prefix, curr_component, cells_to_components, components_to_cells
):
    for cell, cell_component in cells_to_components[curr_component].items():
        if cell_component not in components_to_cells:
            components_to_cells[cell_component] = [f"{prefix}.{cell}"]
        else:
            components_to_cells[cell_component].append(f"{prefix}.{cell}")
        build_components_to_cells(
            prefix + f".{cell}",
            cell_component,
            cells_to_components,
            components_to_cells,
        )


"""
Reads json generated by component-cells backend to produce a mapping from all components
to cell names they could have.

NOTE: Cell names by this point don't contain the simulator-specific prefix. This will be
filled by VCDConverter.enddefinitions().
"""


def read_component_cell_names_json(json_file):
    cell_json = json.load(open(json_file))
    # For each component, contains a map from each cell name to its corresponding component
    # component name --> { cell name --> component name }
    cells_to_components = {}
    main_component = ""
    for curr_component_entry in cell_json:
        cell_map = {}  # mapping cell names to component names for all cells in the current component
        if curr_component_entry["is_main_component"]:
            main_component = curr_component_entry["component"]
        for cell_info in curr_component_entry["cell_info"]:
            cell_map[cell_info["cell_name"]] = cell_info["component_name"]
        cells_to_components[curr_component_entry["component"]] = cell_map
    components_to_cells = {
        main_component: [main_component]
    }  # come up with a better name for this
    build_components_to_cells(
        main_component, main_component, cells_to_components, components_to_cells
    )
    # semi-fully_qualified_cell_name --> component name (of cell)
    # I say semi-here because the prefix depends on the simulator + OS
    # (ex. "TOP.toplevel" for Verilator on ubuntu)
    cell_names_to_components = {}
    for component in components_to_cells:
        for cell in components_to_cells[component]:
            cell_names_to_components[cell] = component

    return main_component, cell_names_to_components, components_to_cells


"""
# Returns { cell --> fsm fully qualified names }
Returns a set of all fsms with fully qualified fsm names
"""


def read_tdcc_file(fsm_json_file, components_to_cells):
    json_data = json.load(open(fsm_json_file))
    fully_qualified_fsms = set()
    par_info = {}  # fully qualified par name --> [fully-qualified par children name]
    reverse_par_info = {}  # fully qualified par name --> [fully-qualified par parent name]
    cell_to_pars = {}
    cell_to_groups_to_par_parent = {}  # cell --> { group --> name of par parent group}. Kind of like reverse_par_info but for normal groups
    # this is necessary because if a nested par occurs simultaneously with a group, we don't want the nested par to be a parent of the group
    par_done_regs = set()
    component_to_fsm_acc = {component: 0 for component in components_to_cells}
    # pass 1: obtain names of all par groups in each component
    component_to_pars = {}  # component --> [par groups]
    for json_entry in json_data:
        if "Par" in json_entry:
            component = json_entry["Par"]["component"]
            if component in component_to_pars:
                component_to_pars[component].append(json_entry["Par"]["par_group"])
            else:
                component_to_pars[component] = json_entry["Par"]["par_group"]
    # pass 2: obtain FSM register info, par group and child register information
    for json_entry in json_data:
        if "Fsm" in json_entry:
            entry = json_entry["Fsm"]
            fsm_name = entry["fsm"]
            component = entry["component"]
            if (
                component in component_to_fsm_acc
            ):  # skip FSMs from components listed in primitive files (not in user-defined code)
                component_to_fsm_acc[component] += 1
                for cell in components_to_cells[component]:
                    fully_qualified_fsm = ".".join((cell, fsm_name))
                    fully_qualified_fsms.add(fully_qualified_fsm)
        if "Par" in json_entry:
            entry = json_entry["Par"]
            par = entry["par_group"]
            component = entry["component"]
            child_par_groups = []
            for cell in components_to_cells[component]:
                fully_qualified_par = ".".join((cell, par))
                if cell in cell_to_pars:
                    cell_to_pars[cell].add(fully_qualified_par)
                else:
                    cell_to_pars[cell] = {fully_qualified_par}
                for child in entry["child_groups"]:
                    child_name = child["group"]
                    if child_name in component_to_pars[component]:
                        fully_qualified_child_name = ".".join((cell, child_name))
                        child_par_groups.append(fully_qualified_child_name)
                        if fully_qualified_child_name in reverse_par_info:
                            reverse_par_info[fully_qualified_child_name].append(
                                fully_qualified_par
                            )
                        else:
                            reverse_par_info[fully_qualified_child_name] = [
                                fully_qualified_par
                            ]
                    else:  # normal group
                        if cell in cell_to_groups_to_par_parent:
                            if child_name in cell_to_groups_to_par_parent[cell]:
                                cell_to_groups_to_par_parent[cell][child_name].add(par)
                            else:
                                cell_to_groups_to_par_parent[cell][child_name] = {par}
                        else:
                            cell_to_groups_to_par_parent[cell] = {child_name: {par}}
                    # register
                    child_pd_reg = child["register"]
                    par_done_regs.add(".".join((cell, child_pd_reg)))
                par_info[fully_qualified_par] = child_par_groups

    return (
        fully_qualified_fsms,
        component_to_fsm_acc,
        par_info,
        reverse_par_info,
        cell_to_pars,
        par_done_regs,
        cell_to_groups_to_par_parent,
    )
