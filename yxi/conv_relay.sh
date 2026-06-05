#!/bin/sh

# convert polybench expects

for file in tests/correctness/relay/*.relay
do
  echo $file;
  fname=$(basename $file .expect);
  fud2 $file --from relay --to calyx -o compiled/$file.futil;
done
