.TOP.TOP.main.systolic_array_component | ({
  "cycles":.clk | add,
  "pe_00": .pe_0_0.acc.out | unique,
  "pe_01": .pe_0_1.acc.out | unique,
  "pe_10": .pe_1_0.acc.out | unique,
  "pe_11": .pe_1_1.acc.out | unique,
})
