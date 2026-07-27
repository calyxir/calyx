#!/bin/sh

rm -rf expect

mkdir expect
mkdir expect/static-tree-edge
for file in ../../tests/correctness/static-tree-edge/*.futil
do
  echo $file;
  fname=$(basename $file .futil);
  dname=$(dirname $file);
  cat $dname/$fname.expect | jq ".memories" -S > expect/static-tree-edge/$fname.expect;
done


mkdir expect/static-control
file="../../tests/correctness/static-control/par-repeat.futil"
echo $file;
fname=$(basename $file .futil);
dname=$(dirname $file);
cat $dname/$fname.expect | jq ".memories" -S > expect/static-control/$fname.expect;


mkdir expect/dahlia
for file in ../../examples/dahlia/*.fuse
do
  echo $file;
  fname=$(basename $file .fuse);
  dname=$(dirname $file);
  cat $dname/$fname.expect | jq ".memories" -S > expect/dahlia/$fname.expect;
done

mkdir expect/ntt
for file in ../../tests/correctness/ntt-pipeline/*.txt
do
  echo $file;
  fname=$(basename $file .txt);
  dname=$(dirname $file);
  cat $dname/$fname.expect | jq ".memories" -S > expect/ntt/$fname.expect;
done

mkdir expect/exp
for file in ../../tests/correctness/exp/*.txt
do
  echo $file;
  fname=$(basename $file .txt);
  dname=$(dirname $file);
  cat $dname/$fname.expect | jq ".memories" -S > expect/exp/$fname.expect;
done

