# memory dimension flattener

this tool can help reduce multi-dimensional memories to a single-dimensional one.

the multi-dimensional memory primitives rely on an underlying single-dimensional memory, plus some address mapping logic to convert multiple addresses to a single one. this tool generates a component to this effect.

it uses the yxi memory data format, which can be generated using the ``yxi`` tool in this folder. after, run:
```
  python3 tools/mem_flat/flatten.py {filename}.yxi
```

to use the generated components:
1. substitute all multi-dimensional memories with a single-dimension one of the correct size.
2. add the generated component, and create an instance of the appropriate type for each multi-dimensional memory. for example, for ``mem_a = seq_mem_d4(32,3,2,1,4,2,2,1,3)``, create a ``mem_a_wrap = d4_32_3x2x1x4`` and replace ``mem_a`` with a 1d mem ``seq_mem_d1(32,24,5)``.
3. search for accesses to the memory. replace all address assignments (to ``addr0, addr1``, etc) with assignments to the ports of the address wrapper. for example, ``mem_a.addr0 = foo`` becomes ``mem_a_wrap.addr0 = foo``.
4. after the address assignments, add a line like ``mem_a.addr0 = mem_a_wrap.addr_o``, connecting the output of the address wrapper to the flattened mem.

note that the address lines of the generated component are marked ``write_together``. while not a strict requirement due to its combinational implementation, this better conforms with the behaviour of the existing multi-dimensional memory primitives.

an example is provided in this folder, which can be run with:
```
   fud2 --from calyx --to jq --through verilator -s sim.data={$PWD}/seq-mem-flat-add.futil.data -s verilog.cycle_limit=500 -s jq.expr=".memories" {$PWD}/seq-mem-flat-add.futil
```
