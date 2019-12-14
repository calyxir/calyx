
%.vcd:
	echo $*
	verilator -cc --trace $*.v --exe tb.cpp
	make -j -C obj_dir -f V$*.mk V$*
	obj_dir/V$*
	rm -rf obj_dir
