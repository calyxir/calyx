# Fuzz: Example
This is a feature for fud. It is a tool that automates the process
of comparing either two input files, or two backend tools.

The input file should be convertible to Calyx, and any backends should be defined in fud to simulate/execute a program.
For the compare file functionality, two input files to be compared and a data template file are mandatory, while an input for backend tool and number of iteration are optional (icarus-verilog is the default backend tool).
For the compare backend functionality, an input file as reference, a data template, and two backend tools mandatory, but the number of iteration is optional.

## Compare Files
To compare two files, the command ``file`` will be used. 
```
python do_fuzz.py file -input_1 <file 1> -input_2 <file 2> -backend <backend tool> -dat <data template> -itr <iteration>
```
As an example, files ``std_add.futil`` and ``std_sub.futil`` using data template ``add_data_template.json`` are compared, with icarus-verilog as backend tool. There will be two iterations.

The command should be:
```
python do_fuzz.py file -input_1 std_add.futil -input_2 std_sub.futil -backend icarus-verilog -dat add_data_template.json -itr 2
```

## Compare Backends
To compare two backend tools, the command ``backend`` will be used.
```
python do_fuzz.py backend -input <file> -backend_1 <backend tool 1> -backend_2 <backend tool 2> -dat <data template> -itr <iteration>
```
As an example, backend tools ``icarus-verilog`` and ``verilog`` are compared, with file ``std_add.futil`` and data template ``add_data_template.json`` as reference. There will be one iterations.

The command should be:
```
python do_fuzz.py backend -input std_add.futil -backend_1 icarus-verilog -backend_2 verilog -dat add_data_template.json -itr 1
```
