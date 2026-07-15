#!/bin/sh

# these scripts can then be piped into /dev/null when actually running


fud2 compiled/$1-axi-wrapped.futil --from calyx --to verilog-noverify -o compiled/$1-axi-wrapped.v

res=$(fud2 compiled/$1-axi-wrapped.v --from verilog-noverify --to cocotb-axi --set sim.data=$2)

if [[ $res ]]; then
  echo "$res"
  exit 0
else
  echo "something went wrong with cocotb"
  exit 1
fi
