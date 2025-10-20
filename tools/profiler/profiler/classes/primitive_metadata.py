from collections import defaultdict
from dataclasses import dataclass
from profiler.classes.errors import ProfilerException

@dataclass
class PrimitiveMetadata:
    """
    Recording types of primitives
    """

    # NOTE: maybe come back to this
    # component --> primitive cell --> primitive name
    p_map: defaultdict[str, defaultdict[str, str]]

    def __init__(self):
        self.p_map = defaultdict(lambda: defaultdict(str))

    def add_entry(self, component: str, primitive_cell: str, primitive_type: str):
        if component in self.p_map:
            inner_dict = self.p_map[component]
            if primitive_cell in inner_dict and inner_dict[primitive_cell] != primitive_type:
                raise ProfilerException("Conflict in PrimitiveMetadata dictionary!")
        else:
            inner_dict = defaultdict()
            self.p_map[component] = inner_dict
        inner_dict[primitive_cell] = primitive_type

    def obtain_entry(self, component: str, primitive_cell: str):
        if primitive_cell not in self.p_map[component]:
            raise ProfilerException(f"{primitive_cell} is not in the primitive metadata for component {component}")
        return self.p_map[component][primitive_cell]

