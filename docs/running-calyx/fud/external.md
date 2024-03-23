# External Stages

`fud` supports using stages that aren't defined in its main source tree.
These are known as 'external stages' and the provide a mechanism
for projects using Calyx to take advantage of `fud`. You can register an
external stage with:
```
fud register stage_name -p /path/to/stage.py
```
Once an external stage is registered, it behaves exactly like any other stage.

You can remove an external stage with:
```
fud register stage_name --delete
```

The following defines a stage that transforms [MrXL][] programs to Calyx
programs.

```python
{{#include ../../frontends/mrxl/fud/mrxl.py}}
```

External stages *must* define default values for configuration keys using the
`Stage.defaults()` static method and the name of the stage using the static
`name` field.

## Stage Configuration

Like normal stages, external stages can have persistent configuration
information saved using `fud config`.

To add persistent stage configuration, run:
```
fud config stages.<stage-name>.<key> <value>
```

To dynamically override the value of a field during execution, use the `-s
flag`:

```
fud e -s <stage-name>.<key> <value> ...
```

The override order for stage configuration is:
1. Dynamic values provided by `-s`.
2. Configuration value in the fud config.
3. Default value provided by `Stage.defaults()`

[MrXL]: ../../frontends/mrxl.md
