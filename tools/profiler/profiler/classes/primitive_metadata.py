from dataclasses import dataclass, field
from profiler.classes.errors import ProfilerException

@dataclass
class PrimitiveMetadata:
    """
    Recording types of primitives
    """

    # # fully_qualified_cell --> primitive name
    # p_map: dict[str, str] = field(default_factory=dict)

    # NOTE: maybe come back to this
    # component --> primitive cell --> primitive name
    p_map: dict[str, dict[str, str]] = field(default_factory=dict)


