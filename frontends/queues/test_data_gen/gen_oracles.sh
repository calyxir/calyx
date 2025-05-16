cat ../tests/strict/strict_3flow_test.data \
  | python3 compile_oracles.py 20000 16 strict_3flows.json --keepgoing \
  > ../tests/strict/strict_3flow_test.expect
[[ $? -eq 0 ]] && echo "Generated strict_3flows.expect"

cat ../tests/round_robin/rr_2flow_test.data \
  | python3 compile_oracles.py 20000 16 rr_2flows.json --keepgoing \
  > ../tests/round_robin/rr_2flow_test.expect
[[ $? -eq 0 ]] && echo "Generated rr_2flows.expect"

cat ../tests/complex_tree_test.data \
  | python3 compile_oracles.py 20000 16 complex_tree.json --keepgoing \
  > ../tests/complex_tree_test.expect
[[ $? -eq 0 ]] && echo "Generated complex_tree_test.expect"

cat ../tests/fifo_test.data \
  | python3 compile_oracles.py 20000 16 fifo.json --keepgoing \
  > ../tests/fifo_test.expect
[[ $? -eq 0 ]] && echo "Generated fifo_test.expect"

cat ../tests/pifo_tree_test.data \
  | python3 compile_oracles.py 20000 16 pifo_tree.json --keepgoing \
  > ../tests/pifo_tree_test.expect
[[ $? -eq 0 ]] && echo "Generated pifo_tree_test.expect"