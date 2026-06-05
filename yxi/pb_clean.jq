# rudimentary tool for aligning the polybench expects with cocotb output

. | to_entries
| map(
  .key as $k | .value as $v |
  # flatten only 2d or above arrays
  if ($v[0] | type == "array") then
    .key |= . + "0"|
    .value |= (. | flatten)
  end
)
| from_entries
