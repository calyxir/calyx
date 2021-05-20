import simplejson as sjson
from memory import MemoryData, Format, Serializable, contains_keys
from jsonschema import validate


class JSONSerializable(Serializable):
    def __init__(self):
        super().__init__()

    def deserialize(self, obj):
        if isinstance(obj, Format):
            return {
                "numeric_type": obj.numeric_type,
                "is_signed": obj.is_signed,
                "width": obj.width,
            }
        elif isinstance(obj, MemoryData):
            return {"data": obj.data, "format": obj.data_format}
        raise TypeError

    def serialize(self, dct):
        if contains_keys(dct, ["data", "format"]):
            return MemoryData(dct["data"], dct["format"])
        elif contains_keys(dct, ["numeric_type", "is_signed", "width"]):
            return Format(dct["numeric_type"], dct["is_signed"], dct["width"])
        return dct


# memories = {"A0": MemoryData([0, 1, 2, 3], Format("bitnum", False, 32))}
# j = JSONSerializable().dumps(memories)

# j = JSONSerializable().loads(
#     open(
#         "/home/samthomas/Research/futil/tests/correctness/invoke-memory.futil.data"
#     ).read(),
# )
j = sjson.load(open("/home/samthomas/Research/futil/vcopy.data"))

validate(
    j,
    schema=sjson.load(
        open("/home/samthomas/Research/futil/fud/dataformat/data-schema.json")
    ),
)
