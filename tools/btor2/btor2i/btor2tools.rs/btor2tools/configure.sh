#!/usr/bin/env bash

set -e -o pipefail

BUILD_DIR=build

asan=no
check=no
debug=no
static=no
btor2aiger=no

#--------------------------------------------------------------------------#

die () {
  echo "*** configure.sh: $*" 1>&2
  exit 1
}

usage () {
cat <<EOF
usage: ./configure.sh [<option> ...]

where <option> is one of the following:

  -O                optimized compilation (default)
  -g                compile with debugging support
  -c                check assertions even in optimized compilation
  --static          static compilation
  --asan            compile with ASAN support
  --btor2aiger      build btor2aiger binary

You might also want to use the environment variables
CC and CXX to specify the used C and C++ compiler, as in

  CC=gcc-4.4 CXX=g++-4.4 ./configure.sh

which forces to use 'gcc-4.4' and 'g++-4.4'.
EOF
  exit 0
}

#--------------------------------------------------------------------------#

while [ $# -gt 0 ]
do
  case $1 in
    -g) debug=yes;;
    -O) debug=no;;
    -c) check=yes;;
    --static) static=yes;;
    --asan) asan=yes;;
    --btor2aiger) btor2aiger=yes;;
    -h|-help|--help) usage;;
    -*) die "invalid option '$1' (try '-h')";;
  esac
  shift
done

#--------------------------------------------------------------------------#

rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}"

cmake_opts=""
[ $debug = yes ] && cmake_opts="$cmake_opts -DCMAKE_BUILD_TYPE=Debug"
[ $check = yes ] && cmake_opts="$cmake_opts -DCHECK=ON"
[ $static = yes ] && cmake_opts="$cmake_opts -DBUILD_SHARED_LIBS=OFF"
[ $asan = yes ] && cmake_opts="$cmake_opts -DASAN=ON"
[ $btor2aiger = yes ] && cmake_opts="$cmake_opts -DBUILD_BTOR2AIGER=ON"

cd "${BUILD_DIR}"
cmake .. $cmake_opts
