#!/usr/bin/awk -f

/run failed/,/make/ {
  print
}

/Output:/,/run passed/{
  gsub(/Output.*$/, "", $0)
  gsub(/.*run passed.*$/, "", $0)
  print
}
