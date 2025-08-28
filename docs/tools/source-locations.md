# Source Location Metadata

Calyx is an intermediate language (IL) for Accelerator Design
Languages (ADLs), higher-level languages that target hardware. It is
helpful to be able to map Calyx code to the original ADL code. The
Source Location Metadata format is used by the Calyx compiler and
tools to map back Calyx to ADL.

The source location comes in three parts:
1. Components, cells, groups, and control blocks/statements contain the `pos` attribute that tracks a unique **position ID**.

2. A `FILES` table that maps unique **file IDs** to corresponding files.

3. A `POSITIONS` table that maps position IDs to file IDs and the line numbers.

The `FILES` and `POSITIONS` tables are in the `sourceinfo` metadata block at the end of the Calyx file.

For example, here is a shortened Calyx program with source location metadata:
```
component main<"pos"={0}>() -> () {
  cells { ... }
  wires {
    group write<"pos"={1}> { ... }
    ...
  }
  control {
    @pos{2} write;
  }
}

sourceinfo #{
FILES
  0: source.py
POSITIONS
  0: 0 6
  1: 0 15
  2: 0 25
  ...
}
```

Here, the component `main` has a position ID 0, the group `write` has a position ID 1, and the control statement enabling `write` has a position ID 2. The `FILES` map maps the file ID 0 to `source.py`. From this information, we can use the `POSITIONS` map to find that `main` was defined in `source.py` line 6, the group `write` was defined in `source.py` line 15, and the enable for `write` was defined in `source.py` line 25.

### Calyx positions

In order to track source locations of a Calyx program, use the `metadata-table-generation` pass.