import os

INVISIBLE = "gray"
ACTIVE_CELL_COLOR = "pink"
ACTIVE_GROUP_COLOR = "mediumspringgreen"
ACTIVE_PRIMITIVE_COLOR = "orange"
TREE_PICTURE_LIMIT = 300


def create_edge_dict(path_dict):
    path_to_edges = {}  # stack list string --> [edge string representation]
    all_edges = set()

    for path_id in path_dict:
        path = path_dict[path_id]
        edge_set = []
        for i in range(len(path) - 1):
            edge = f"{path[i]} -> {path[i + 1]}"
            edge_set.append(edge)
            all_edges.add(edge)
        path_to_edges[path_id] = edge_set

    return path_to_edges, list(sorted(all_edges))


def create_tree(timeline_map):
    """
    Creates a tree that encapsulates all stacks that occur within the program.
    """
    node_id_acc = 0
    tree_dict = {}  # node id --> node name
    path_dict = {}  # stack list string --> list of node ids
    path_prefixes_dict = {}  # stack list string --> list of node ids
    stack_list = []
    # collect all of the stacks from the list. (i.e. "flatten" the timeline map values.)
    for sl in timeline_map.values():
        for s in sl:
            if s not in stack_list:
                stack_list.append(s)
    stack_list.sort(key=len)
    for stack in stack_list:
        stack_len = len(stack)
        id_path_list = []
        prefix = ""
        # obtain the longest prefix of the current stack. Everything after the prefix is a new stack element.
        for i in range(1, stack_len + 1):
            attempted_prefix = ";".join(stack[0 : stack_len - i])
            if attempted_prefix in path_prefixes_dict:
                prefix = attempted_prefix
                id_path_list = list(path_prefixes_dict[prefix])
                break
        # create nodes
        if prefix != "":
            new_nodes = stack[stack_len - i :]
            new_prefix = prefix
        else:
            new_nodes = stack
            new_prefix = ""
        for elem in new_nodes:
            if new_prefix == "":
                new_prefix = elem
            else:
                new_prefix += f";{elem}"
            tree_dict[node_id_acc] = elem
            id_path_list.append(node_id_acc)
            path_prefixes_dict[new_prefix] = list(id_path_list)
            node_id_acc += 1
        path_dict[new_prefix] = id_path_list

    return tree_dict, path_dict


def create_tree_rankings(
    trace, tree_dict, path_dict, path_to_edges, all_edges, dot_out_dir
):
    stack_list_str_to_used_nodes = {}
    stack_list_str_to_used_edges = {}
    stack_list_str_to_cycles = {}
    all_nodes = set(tree_dict.keys())

    # accumulating counts
    for i in trace:
        stack_list_str = str(trace[i])
        if stack_list_str in stack_list_str_to_cycles:
            stack_list_str_to_cycles[stack_list_str].append(i)
            continue
        stack_list_str_to_cycles[stack_list_str] = [i]
        used_nodes = set()
        used_edges = set()

        for stack in trace[i]:
            stack_id = ";".join(stack)
            for node_id in path_dict[stack_id]:
                used_nodes.add(node_id)
            for edge in path_to_edges[stack_id]:
                used_edges.add(edge)
        stack_list_str_to_used_nodes[stack_list_str] = used_nodes
        stack_list_str_to_used_edges[stack_list_str] = used_edges

    sorted_stack_list_items = sorted(
        stack_list_str_to_cycles.items(), key=(lambda item: len(item[1])), reverse=True
    )
    acc = 0
    rankings_out = open(os.path.join(dot_out_dir, "rankings.csv"), "w")
    rankings_out.write("Rank,#Cycles,Cycles-list\n")
    for stack_list_str, cycles in sorted_stack_list_items:
        if acc == 5:
            break
        acc += 1
        # draw the tree
        fpath = os.path.join(dot_out_dir, f"rank{acc}.dot")
        with open(fpath, "w") as f:
            f.write("digraph rank" + str(acc) + " {\n")
            # declare nodes.
            for node in all_nodes:
                if node in stack_list_str_to_used_nodes[stack_list_str]:
                    f.write(f'\t{node} [label="{tree_dict[node]}"];\n')
                else:
                    f.write(
                        f'\t{node} [label="{tree_dict[node]}",color="{INVISIBLE}",fontcolor="{INVISIBLE}"];\n'
                    )
            # write all edges.
            for edge in all_edges:
                if edge in stack_list_str_to_used_edges[stack_list_str]:
                    f.write(f"\t{edge} ; \n")
                else:
                    f.write(f'\t{edge} [color="{INVISIBLE}"]; \n')
            f.write("}")

        rankings_out.write(f"{acc},{len(cycles)},{';'.join(str(c) for c in cycles)}\n")


# one tree to summarize the entire execution.
def create_aggregate_tree(timeline_map, out_dir, tree_dict, path_dict):
    path_to_edges, all_edges = create_edge_dict(path_dict)

    leaf_nodes_dict = {
        node_id: 0 for node_id in tree_dict
    }  # how many times was this node a leaf?
    edges_dict = {}  # how many times was this edge active?

    for stack_list in timeline_map.values():
        edges_this_cycle = set()
        leaves_this_cycle = set()
        stacks_this_cycle = set(map(lambda stack: ";".join(stack), stack_list))
        for stack in stack_list:
            stack_id = ";".join(stack)
            for edge in path_to_edges[stack_id]:
                if edge not in edges_this_cycle:
                    if edge not in edges_dict:
                        edges_dict[edge] = 1
                    else:
                        edges_dict[edge] += 1
                    edges_this_cycle.add(edge)
            # record the leaf node. ignore all primitives as I think we care more about the group that called the primitive (up to debate)
            leaf_node = path_dict[stack_id][-1]
            if "primitive" in tree_dict[leaf_node]:
                leaf_node = path_dict[stack_id][-2]
                leaf_id = ";".join(stack[:-1])
                # if the current stack (minus primitive) is a prefix of another stack, then we shouldn't count it in as a leaf node.
                contained = False
                for other_stack in stacks_this_cycle:
                    if other_stack != stack_id and leaf_id in other_stack:
                        contained = True
                        break
                if contained:  # this is not actually a leaf node, so we should move onto the next leaf node.
                    continue
            if leaf_node not in leaves_this_cycle:
                leaf_nodes_dict[leaf_node] += 1
                leaves_this_cycle.add(leaf_node)

    # write the tree
    with open(os.path.join(out_dir, "aggregate.dot"), "w") as f:
        f.write("digraph aggregate {\n")
        # declare nodes
        for node in leaf_nodes_dict:
            if "primitive" in tree_dict[node]:
                f.write(
                    f'\t{node} [label="{tree_dict[node]}", style=filled, color="{ACTIVE_PRIMITIVE_COLOR}"];\n'
                )
            elif "[" in tree_dict[node] or "main" == tree_dict[node]:
                f.write(
                    f'\t{node} [label="{tree_dict[node]} ({leaf_nodes_dict[node]})", style=filled, color="{ACTIVE_CELL_COLOR}"];\n'
                )
            else:
                f.write(
                    f'\t{node} [label="{tree_dict[node]} ({leaf_nodes_dict[node]})", style=filled, color="{ACTIVE_GROUP_COLOR}"];\n'
                )
        # write edges with labels
        for edge in edges_dict:
            f.write(f'\t{edge} [label="{edges_dict[edge]}"]; \n')
        f.write("}")


def create_slideshow_dot(timeline_map, dot_out_dir, flame_out_file, flames_out_dir):
    if not os.path.exists(dot_out_dir):
        os.mkdir(dot_out_dir)

    # only produce trees for every cycle if we don't exceed TREE_PICTURE_LIMIT
    if len(timeline_map) > TREE_PICTURE_LIMIT:
        print(
            f"Simulation exceeds {TREE_PICTURE_LIMIT} cycles, skipping slideshow trees for every cycle..."
        )
        return
    tree_dict, path_dict = create_tree(timeline_map)
    path_to_edges, all_edges = create_edge_dict(path_dict)

    for i in timeline_map:
        used_edges = {}
        used_paths = set()
        used_nodes = set()
        all_nodes = set(tree_dict.keys())
        # figure out what nodes are used and what nodes aren't used
        for stack in timeline_map[i]:
            stack_id = ";".join(stack)
            used_paths.add(stack_id)
            for node_id in path_dict[stack_id]:
                used_nodes.add(node_id)
            for edge in path_to_edges[stack_id]:
                if edge not in used_edges:
                    used_edges[edge] = 1
                else:
                    used_edges[edge] += 1

        fpath = os.path.join(dot_out_dir, f"cycle{i}.dot")
        with open(fpath, "w") as f:
            f.write("digraph cycle" + str(i) + " {\n")
            # declare nodes.
            for node in all_nodes:
                if node in used_nodes:
                    f.write(f'\t{node} [label="{tree_dict[node]}"];\n')
                else:
                    f.write(
                        f'\t{node} [label="{tree_dict[node]}",color="{INVISIBLE}",fontcolor="{INVISIBLE}"];\n'
                    )
            # write all edges.
            for edge in all_edges:
                if edge in used_edges.keys():
                    f.write(f"\t{edge} ; \n")
                else:
                    f.write(f'\t{edge} [color="{INVISIBLE}"]; \n')
            f.write("}")
