# FSM to DOT Converter

A tool to visualize Calyx FSM (Finite State Machine) representations as graphs using the DOT format.

## Usage

```bash
# Build the tool
cargo build -p fsm_to_dot

# Run on a Calyx file (output to stdout)
cargo run -p fsm_to_dot -- path/to/file.futil

# Specify a component (default is "main")
cargo run -p fsm_to_dot -- path/to/file.futil -c my_component

# Save output to a file
cargo run -p fsm_to_dot -- path/to/file.futil -o output.dot

# Specify library path
cargo run -p fsm_to_dot -- path/to/file.futil -l ./primitives
```

## Visualizing the Output

Once you have the DOT file, you can visualize it using:

1. **Graphviz** (command line):
   ```bash
   dot -Tpng output.dot -o graph.png
   dot -Tsvg output.dot -o graph.svg
   ```

2. **Online viewers**:
   - [GraphvizOnline](https://dreampuf.github.io/GraphvizOnline/)
   - [Edotor](https://edotor.net/)

3. **VS Code extensions**:
   - Graphviz Preview
   - Graphviz (dot) language support

## Example

```bash
# Generate FSM visualization for a static program
cargo run -p fsm_to_dot -- examples/futil/static.futil -o fsm.dot

# View it with xdot (interactive viewer)
xdot fsm.dot

# Or convert to PNG
dot -Tpng fsm.dot -o fsm.png
```

## How It Works

This tool:
1. Parses the input Calyx file
2. Runs static compilation passes to generate FSMs
3. Extracts FSM structures from the specified component
4. Converts each FSM to DOT format showing:
   - States as nodes (S0, S1, S2, ...)
   - Transitions as edges (with conditions if applicable)
5. Outputs the DOT representation for visualization

## Notes

- Make sure your Calyx program uses static control constructs (static_seq, static_par, static_repeat, etc.)
- The tool runs the `static-compilation` passes to generate FSMs
- If no FSMs are found, check that your program uses static control and that the compilation passes succeeded
