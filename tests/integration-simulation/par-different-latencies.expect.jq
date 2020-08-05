.TOP.main
| ({
  "cycles":.clk | add,
  "x": .one.x_out | .[-1],
  "y": .two.y_out | .[-1],
  "z": .two.z_out | .[-1],
})
