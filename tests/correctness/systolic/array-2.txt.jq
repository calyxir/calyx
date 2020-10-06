.TOP.main | ({ 
  "cycles":.clk | add,
  "out_00": .out_mem["mem(0)(0)"] | .[-1],
  "out_01": .out_mem["mem(0)(1)"] | .[-1],
  "out_10": .out_mem["mem(1)(0)"] | .[-1],
  "out_11": .out_mem["mem(1)(1)"] | .[-1],
  "pe_00": .pe_00.acc.out | unique,
  "pe_01": .pe_01.acc.out | unique,
  "pe_10": .pe_10.acc.out | unique,
  "pe_11": .pe_11.acc.out | unique,
})
