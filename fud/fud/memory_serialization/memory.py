from __future__ import annotations
from dataclasses import dataclass
from typing import List, Dict
import simplejson as sjson


@dataclass
class Format:
    """Represents the storage format of numbers"""

    numeric_type: str
    is_signed: bool
    width: int


@dataclass
class MemoryData:
    """Represents the serialized contents of a memory"""

    data: List[int]
    data_format: Format


def contains_keys(dct, keys):
    for k in keys:
        if k not in dct:
            return False
    return True


class Serializable:
    def __init__(self):
        pass

    def deserialize(self, str) -> Dict[str, MemoryData]:
        raise NotImplementedError

    def serialize(self, dct: Dict[str, MemoryData]) -> str:
        raise NotImplementedError

    def dumps(self, obj):
        return sjson.dumps(obj, indent=2, use_decimal=True, default=self.deserialize)

    def loads(self, data: str) -> Dict[str, MemoryData]:
        return sjson.loads(data, use_decimal=True, object_hook=self.serialize)
