.TOP.main
| ({
  "cycles":.clk | add,
  "x": .x0.out | .[-1],
  "y": .y0.out | .[-1],
  "z": .z0.out | .[-1],
  "i": .i0.out | .[-1],
})
