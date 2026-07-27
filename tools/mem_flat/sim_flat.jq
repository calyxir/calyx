# after obtaining the .memories field of a simulation result (with either icarus / verilator), this tool can be used to flatten higher-dimensioned memories.
# similar to data_flat.jq, but operates on something without metadata

. | to_entries
| map(
  .key as $k | .value as $v |
  # flatten only 2d or above arrays
  if ($v[0] | type == "array") then
    .key |= "flat_" + . |
    .value |= (. | flatten)
  end
)
| from_entries
