from collections import namedtuple
import networkx as nx

from fud.errors import UndefinedStage, MultiplePaths

Edge = namedtuple("Edge", ["dest", "stage"])


class Registry:
    """
    Defines all the stages and how they transform files from one stage to
    another.
    """

    def __init__(self, config):
        self.config = config
        self.graph = nx.DiGraph()

    def register(self, stage, src=None, tar=None):
        """
        Defines a new stage named `stage` that converts programs from `src` to
        `tar`
        """
        if src is None:
            src = stage.name
        if tar is None:
            tar = stage.target_stage

        self.graph.add_edge(src, tar, stage=stage)

    def make_path(self, start, dest, through=[]):
        """
        Compute a path from `start` to `dest` that contains all stages
        mentioned in `through`.
        Raises MultiplePaths if there is more than one matching path for the
        (start, dest) pair.
        """

        nodes = self.graph.nodes()
        if start not in nodes:
            raise UndefinedStage(start)

        if dest not in nodes:
            raise UndefinedStage(dest)

        all_paths = list(nx.all_simple_edge_paths(self.graph, start, dest))

        # Compute all stage pipelines that can be run.
        stage_paths = []
        # Minimum cost path
        min_cost = None
        for path in all_paths:
            through_check = set(through)
            stage_path = []
            # Cost of the Path
            path_cost = None
            for (src, dst) in path:
                if src in through_check:
                    through_check.remove(src)
                stage = self.graph.get_edge_data(src, dst)["stage"]
                stage_path.append(stage)
                # Get the cost of the path if there is any
                #  print(self.config.get(("stages", stage.name, "priority")))
                cost = self.config.get(("stages", stage.name, "priority"))
                if cost is not None:
                    if path_cost is None:
                        path_cost = cost
                    else:
                        path_cost += cost

            # If there are no items left in the --through check then add it
            if len(through_check) == 0:
                # If this path has a cost, then stage_paths can only have
                # one path in it.
                if path_cost is not None:
                    if min_cost is None or path_cost < min_cost:
                        stage_paths = [stage_path]
                    elif min_cost == path_cost:
                        stage_paths.append(stage_path)
                    min_cost = path_cost
                elif min_cost is None:
                    stage_paths.append(stage_path)

        if len(stage_paths) > 1:
            p = []
            for path in all_paths:
                if len(path) == 0:
                    continue
                # Add the starting src
                path_str = path[0][0]
                for (_, dst) in path:
                    path_str += f" → {dst}"
                    cost = self.config.get(("stages", dst, "priority"))
                    if cost is not None:
                        path_str += f" (cost: {cost})"
                p.append(path_str)
            raise MultiplePaths(start, dest, "\n".join(p))
        elif len(stage_paths) == 0:
            return None
        else:
            return stage_paths[0]

    def __str__(self):
        stages = {}
        transforms = []

        for (src, dst, attr) in sorted(self.graph.edges(data=True)):
            transforms.append((src, dst, attr["stage"].description))
            if src not in stages:
                stages[src] = []
            stages[src].append(dst)

        all_stages = ""
        for (src, dsts) in stages.items():
            d = ", ".join(dsts)
            all_stages += f"\n{src} → {d}"

        all_transforms = "\n".join([f"{s} → {e}: {d}" for (s, e, d) in transforms])

        return f"""List of possible stage transformations: {all_stages}

Legend:
{all_transforms}
"""
