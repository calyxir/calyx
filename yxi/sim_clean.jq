. | to_entries
| map(
  .key as $k | .value as $v |
  .value |= $v.data
)
| from_entries
