#!/bin/sh
readonly SCRIPTDIR=$(dirname "$(readlink -f $0)")
readonly ROOTDIR=$SCRIPTDIR/..
readonly BUILDDIR=$ROOTDIR/build
readonly PARSERDIR=$ROOTDIR/src/btor2parser
HTMPFILE=/tmp/btor2parser-run-coverage-tags-in-header
PTMPFILE=/tmp/btor2parser-run-coverage-tags-in-parsed
grep BTOR2_FORMAT_TAG_ $PARSERDIR/btor2parser.h | \
sed -e 's,.*TAG_,,' -e 's/,.*$//g' | \
sort > $HTMPFILE
grep 'PARSE (' $PARSERDIR/btor2parser.c | \
sed -e 's,.*PARSE (,,' -e 's/,.*//g' | \
sort > $PTMPFILE
diff $HTMPFILE $PTMPFILE | sed -e '/^[0-9]/d'
cd $SCRIPTDIR/../
make clean
$ROOTDIR/configure.sh -g -c -gcov
make
cd -
$SCRIPTDIR/runtests.sh
gcov -o $BUILDDIR $PARSERDIR/btor2parser.c
echo "vi btor2parser.c.gcov"
