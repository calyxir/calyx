#!/bin/sh

readonly SCRIPTDIR=$(dirname "$(readlink -f $0)")
readonly BINDIR=$SCRIPTDIR/../bin

readonly GREEN='\033[0;32m'
readonly RED='\033[0;31m'
readonly NOCOLOR='\033[0m'

ok=0
failed=0
total=0

erase ()
{
  echo -en "\r                                                               \r"
}

runtest ()
{
  echo -n "$1 ..."
  rm -f $1.log
  $BINDIR/catbtor $1.in 1>$1.log 2>&1
  if diff $1.log $1.out 1>/dev/null 2>/dev/null
  then
    echo -en "${GREEN} ok${NOCOLOR}\r"
    ok=`expr $ok + 1`
    erase
  else
    echo -e "${RED} failed${NOCOLOR}"
    failed=`expr $failed + 1`
  fi
  total=`expr $total + 1`
}

cd $SCRIPTDIR/tests || exit 1

for i in *.in
do
  name=`basename $i .in`
  runtest $name
done

echo
if [ $ok -ne 0 ]
then
  echo -e "${GREEN}$ok ok ${NOCOLOR}"
fi
if [ $failed -ne 0 ]
then
  echo -e "${RED}$failed failed${NOCOLOR}"
fi
echo "$total total"

