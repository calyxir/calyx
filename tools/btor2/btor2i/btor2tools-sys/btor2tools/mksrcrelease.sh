#!/bin/sh

force=no

die () {
  echo "*** mksrcrelease.sh: $*" 1>&2
  exit 1
}

[ -e src ] || die "can not find 'src', call from btor2tools base directory"

while [ $# -gt 0 ]
do
  case $1 in
    -h) echo "usage: mksrcrelease.sh [-h][-f]";exit 0;;
    -f) force=yes;;
    *) die "invalid command line option '$1'";;
  esac
  shift
done

LC_TIME="en_US.UTF-8"
export LC_TIME

date=`date +%y%m%d`
version=`cat VERSION`
gitid=`git rev-parse HEAD`
gitid_short=`git rev-parse --short=7 HEAD`

id="$version-$gitid_short-$date"
name=btor2tools-$id
dir="/tmp/$name"

if [ -d $dir ]
then
  [ $force = no ] && die "$dir already exists, use '-f'"
fi

rm -rf $dir
mkdir $dir || exit 1

mkdir $dir/src || exit 1

cp -p \
  AUTHORS \
  VERSION \
  LICENSE.txt \
  README.md \
  configure.sh \
  makefile.in \
$dir/

cp -p --parents \
  src/btor2parser/btor2parser.[ch] \
  src/btorsim/btorsim.c \
  src/btorsim/btorsimbv.[ch] \
  src/btorsim/btorsimrng.[ch] \
  src/util/btor2mem.h \
  src/util/btor2stack.h \
  src/catbtor.c \
$dir

cp -p -r --parents \
  examples/btorsim/*.btor2 \
  examples/btorsim/mc-witnesses \
  examples/btorsim/run-examples.sh \
$dir

cd /tmp/
rm -f $name.tar.xz
tar Jcf $name.tar.xz $name
ls -l /tmp/$name.tar.xz | awk '{print $5, $NF}'
rm -rf $dir
