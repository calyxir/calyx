# rudimentary tool for converting .data files

. | to_entries
| map(
  .key as $k | .value as $v |
  # flatten only 2d or above arrays
  if ($v.data[0] | type == "array") then
    .key |= . + "0"|
    .value.data |= (. | flatten)
  end
)
| from_entries
