.TOP.main | ({
  "cycles":.clk | add,
  "out_00": .out_mem["mem(0)(0)"] | .[-1],
  "out_01": .out_mem["mem(0)(1)"] | .[-1],
  "out_10": .out_mem["mem(1)(0)"] | .[-1],
  "out_11": .out_mem["mem(1)(1)"] | .[-1],
  "pe_00": .pe_0_0.acc.out | unique,
  "pe_01": .pe_0_1.acc.out | unique,
  "pe_10": .pe_1_0.acc.out | unique,
  "pe_11": .pe_1_1.acc.out | unique,
})
