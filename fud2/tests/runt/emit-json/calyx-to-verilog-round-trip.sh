found="$(fud2 fud2/tests/runt/emit-json/calyx.futil --to verilog -m json-plan | fud2 fud2/tests/runt/emit-json/calyx.futil --to verilog --planner json)"
expected="$(fud2 fud2/tests/runt/emit-json/calyx.futil --to verilog)"
if [ "$found" = "$expected" ]; then
  echo "pass"
else
  echo "fail"
fi
