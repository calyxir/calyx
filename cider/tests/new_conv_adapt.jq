
# strips the fud2 format json metadata from entries

. | to_entries
| map(
  .key as $k | .value as $v |
  .value |= $v.data
)
| from_entries

