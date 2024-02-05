#!/usr/bin/env bash

set -e -o pipefail

rm -rf deps
mkdir -p deps

# Setup AIGER
rm -rf aiger-1.9.4.tar.gz
wget http://fmv.jku.at/aiger/aiger-1.9.4.tar.gz
tar xf aiger-1.9.4.tar.gz
mv aiger-1.9.4 deps/aiger


# Setup Boolector
git clone https://github.com/boolector/boolector deps/boolector

cd deps/boolector
git checkout bitblast-api

./contrib/setup-btor2tools.sh
./contrib/setup-lingeling.sh
./configure.sh --prefix $(pwd)/../install
cd build

make -j$(nproc) install
