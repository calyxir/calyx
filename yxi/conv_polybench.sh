#!/bin/sh

# convert polybench expects

for file in ../cider/tests/benchmarks/polybench/*.expect
do
  echo $file;
  fname=$(basename $file .expect);
  jq -f pb_clean.jq -S $file > tests/axi/polybench/$fname.expect;
done
