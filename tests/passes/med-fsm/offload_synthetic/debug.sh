#!/bin/sh
set -e
# set -x

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

FLAGS='-p med-fsm -x tdcc:infer-fsms -p lower'
FLAG_TDCC='-x tdcc:infer-fsms -p lower'
INPUT_RANDOM_DATA_PATH="$SCRIPT_DIR/random.json"
MED_FSM_DESIGN_PATH="$SCRIPT_DIR/offload_less_synthetic_med_fsm.futil"
DESIGN_PATH="$SCRIPT_DIR/offload_less_synthetic.futil"
REDUCE_PATH="$SCRIPT_DIR/reduce_offload_less_synthetic.futil"

fud exec --from calyx --to jq --through icarus-verilog --through dat \
  -s calyx.flags "$FLAGS" \
  -s verilog.data "$INPUT_RANDOM_DATA_PATH" \
  -s jq.expr '.memories' \
  "$DESIGN_PATH" -q

fud exec --from calyx --to jq --through icarus-verilog --through dat \
  -s verilog.data "$INPUT_RANDOM_DATA_PATH" \
  -s jq.expr '.memories' \
  "$DESIGN_PATH" -q

# fud exec --from calyx --to jq --through icarus-verilog --through dat \
#   -s calyx.flags "$FLAG_TDCC" \
#   -s verilog.data "$INPUT_RANDOM_DATA_PATH" \
#   -s jq.expr '.memories' \
#   "$MED_FSM_DESIGN_PATH" -q

fud exec --from calyx --to jq --through icarus-verilog --through dat \
  -s calyx.flags "$FLAGS" \
  -s verilog.data "$INPUT_RANDOM_DATA_PATH" \
  -s jq.expr '.memories' \
  "$REDUCE_PATH" -q

fud exec --from calyx --to jq --through icarus-verilog --through dat \
  -s verilog.data "$INPUT_RANDOM_DATA_PATH" \
  -s jq.expr '.memories' \
  "$REDUCE_PATH" -q

