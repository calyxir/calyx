from typing import List, Tuple

from collections import namedtuple
import networkx as nx  # type: ignore

from fud import stages, errors
from fud.errors import UndefinedState, MultiplePaths

# An edge in the state graph
Edge = namedtuple("Edge", ["dest", "stage"])
DEPRECATED_STATES = [("futil", "calyx")]


class Registry:
    """
    Defines all the stages and how they transform files from one stage to
    another.
    """

    def __init__(self, config):
        self.config = config
        self.graph = nx.DiGraph()

    def get_states(self, stage: str) -> List[Tuple[str, str]]:
        """
        Returns the pairs of input and output states that the given stage
        operates upon.
        """
        out = [
            (s, e) for (s, e, st) in self.graph.edges(data="stage") if st.name == stage
        ]
        assert len(out) > 0, f"No state tranformation for {stage} found."
        return out

    @staticmethod
    def _deprecate_check(stage_name, state):
        for st, alt in DEPRECATED_STATES:
            if state == st:
                raise errors.DeprecatedState(stage_name, state, alt)

    def register(self, stage):
        """
        Defines a new stage named `stage` that converts programs from `src` to
        `tar`
        """

        # Error if the stage is attempting to register deprecated states.
        Registry._deprecate_check(stage.name, stage.src_state)
        Registry._deprecate_check(stage.name, stage.target_state)

        self.graph.add_edge(stage.src_state, stage.target_state, stage=stage)

    def make_path(self, start: str, dest: str, through=[]) -> List[stages.Stage]:
        """
        Compute a path from `start` to `dest` that contains all stages
        mentioned in `through`.
        Raises MultiplePaths if there is more than one matching path for the
        (start, dest) pair.
        """

        nodes = self.graph.nodes()
        if start not in nodes:
            raise UndefinedState(start, "Validate source state of the path")

        if dest not in nodes:
            raise UndefinedState(dest, "Validate target state of the path")

        for node in through:
            if node not in nodes:
                raise UndefinedState(node, "Stage provided using --through")

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
            for src, dst in path:
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
            raise MultiplePaths(start, dest, self.paths_str(all_paths))
        elif len(stage_paths) == 0:
            raise errors.NoPathFound(start, dest, through)
        else:
            return stage_paths[0]

    def paths_str(self, paths):
        """
        Generate a string representation for computed paths
        """
        p = []
        for path in paths:
            if len(path) == 0:
                continue
            # Add the starting src
            path_str = path[0][0]
            for _, dst in path:
                path_str += f" → {dst}"
                cost = self.config.get(("stages", dst, "priority"))
                if cost is not None:
                    path_str += f" (cost: {cost})"
            p.append(path_str)
        return "\n".join(p)

    def all_from(self, from_st):
        """
        Returns all the transformations from a particular state
        """

    def __str__(self):
        stages = {}
        transforms = []

        for src, dst, attr in sorted(self.graph.edges(data=True)):
            transforms.append((src, dst, attr["stage"].name, attr["stage"].description))
            if src not in stages:
                stages[src] = []
            stages[src].append(dst)

        all_stages = ""
        for src, dsts in stages.items():
            d = ", ".join(dsts)
            all_stages += f"\n{src} → {d}"

        all_transforms = "\n".join(
            [f"{s} → {e} ({n}): {d}" for (s, e, n, d) in transforms]
        )

        return f"""List of possible stage transformations: {all_stages}

Legend:
{all_transforms}
"""
