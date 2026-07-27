# rudimentary tool for converting .data files for use with a wrapped design with flat mems.
# essentially just reduces all dimensions to 1, and updates names according to what mem_flat will do.

. | to_entries
| map(
  .key as $k | .value as $v |
  # flatten only 2d or above arrays
  if ($v.data[0] | type == "array") then
    .key |= "flat_" + . |
    .value.data |= (. | flatten)
  end
)
| from_entries
