.TOP.main | ({
  "cycles":.clk | add,
  "out_00": .out_mem["mem(0)(0)"] | .[-1],
  "pe_00": .pe_0_0.acc.out | unique,
})
