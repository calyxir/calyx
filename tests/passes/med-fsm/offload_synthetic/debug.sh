#!/bin/sh
set -e
# set -x

FLAGS='-p med-fsm -x tdcc:infer-fsms -p lower'
INPUT_RANDOM_DATA_PATH=/home/cynyu_s/calyx/tests/passes/med-fsm/offload_synthetic/random.json
DESIGN_PATH=/home/cynyu_s/calyx/tests/passes/med-fsm/offload_synthetic/offload_less_synthetic.futil
REDUCE_PATH=/home/cynyu_s/calyx/tests/passes/med-fsm/offload_synthetic/reduce_offload_less_synthetic.futil

fud exec --from calyx --to jq --through icarus-verilog --through dat \
  -s calyx.flags "$FLAGS" \
  -s verilog.data "$INPUT_RANDOM_DATA_PATH" \
  -s jq.expr '.memories' \
  "$DESIGN_PATH" -q

fud exec --from calyx --to jq --through icarus-verilog --through dat \
  -s calyx.flags "$FLAGS" \
  -s verilog.data "$INPUT_RANDOM_DATA_PATH" \
  -s jq.expr '.memories' \
  "$REDUCE_PATH" -q

fud exec --from calyx --to jq --through icarus-verilog --through dat \
  -s verilog.data "$INPUT_RANDOM_DATA_PATH" \
  -s jq.expr '.memories' \
  "$DESIGN_PATH" -q