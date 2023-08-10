# Multiple Paths

`fud` can define a stage graph that can have multiple paths between the source
and target.
For example, if you register the [Icarus Verilog](./index.md#icarus-verilog) simulator stage, then multiple
paths can be used to generate VCD files from Dahlia programs:

```
% fud e --from dahlia --to vcd
[fud] ERROR: Multiple stage pipelines can transform dahlia to vcd:
dahlia → calyx → verilog → vcd
dahlia → calyx → icarus-verilog → vcd
Use the --through flag to select an intermediate stage
```

`fud` says that both the `verilog` and `icarus-verilog` stages can be used to
generate the VCD file and you need to provide the `--through` flag to decide
which stage to select.

The following command will simulate the program using the `icarus-verilog`
stage:
```
% fud e --from dahlia --to vcd --through icarus-verilog
```

In general, the `--through` flag can be repeated as many times as needed to
get a unique `fud` transformation pipeline.

## Using Stage Priority

If the common workflow uses the same stage every time, it can be annoying to
specify the stage name using the `--through` flag.
You can specify a priority field in the configuration of a stage to ensure
it `fud` automatically selects it when multiple paths exists.

For example, to always select the `verilog` stage, add the priority `1` to the
stage:
```
fud c stages.verilog.priority 1
```

Now, the command `fud e --from dahlia --to vcd` is no longer ambiguous; `fud`
will always choose the `verilog` stage to transform programs from Dahlia
sources to VCD.

In case multiple paths have the same cost, `fud` will again require the
`--through` flag to disambiguate paths.
