from collections import namedtuple


Edge = namedtuple('Edge', ['dest', 'stage'])


# TODO: assuming there is only a single path
class Registry:
    def __init__(self, config):
        self.config = config

        self.nodes = {}

    def register(self, stage, src=None, tar=None):
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
        else:
            if start not in self.nodes:
                # if start no in nodes, then there is no
                # path from start to dest
                return None
            else:
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
        output = []
        for k, v in self.nodes.items():
            vals = [x.dest for x in v]
            output.append(f"{k} -> {vals}")
        return '\n'.join(output)
