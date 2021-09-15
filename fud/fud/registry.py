from collections import namedtuple

Edge = namedtuple("Edge", ["dest", "stage"])


# TODO: assuming there is only a single path
class Registry:
    def __init__(self, config):
        self.config = config
        self.nodes = {}

    def get_nodes(self):
        return self.nodes

    def register(self, stage, src=None, tar=None):
        """
        Defines a new stage named `stage` that converts programs from `src` to
        `tar`
        """
        if src is None:
            src = stage.name
        if tar is None:
            tar = stage.target_stage

        # check if this node is already in the graph
        if src in self.nodes:
            self.nodes[src].append(Edge(tar, stage))
        else:
            self.nodes[src] = [Edge(tar, stage)]

    def make_path(self, start, dest):
        if start == dest:
            # we have reached the destination, start a list
            return []

        if start not in self.nodes:
            # if start no in nodes, then there is no
            # path from start to dest
            return None

        # go through edges in self.nodes[start]
        # recursively calling self.make_path and
        # and only keeping non-none paths
        for edge in self.nodes[start]:
            path = self.make_path(edge.dest, dest)
            if path is not None:
                path.insert(0, edge)
                return path

        # if we haven't found a path, return none
        return None

    def __str__(self):
        transforms = []
        legend = []
        for k, v in self.nodes.items():
            vals = [x.dest for x in v]
            legend += [(k, x.dest, x.stage.description) for x in v]
            transforms.append(f"{k} → {', '.join(vals)}")

        all_transforms = "\n".join(transforms)
        all_stages = "\n".join([f"{s} → {e}: {d}" for (s, e, d) in legend])

        return f"""List of possible stage transformations:
{all_transforms}

Legend:
{all_stages}
"""
