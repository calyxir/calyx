## Structure

- [src](src)
  - [main.rs](src/main.rs)
  - [cmdline.rs](src/cmdline.rs) (parsing the input)
  - [errors.rs](src/errors.rs) (error message generator)
  - [utils.rs](src/main.rs) 
  - [lang/](src/lang/) (building AST from futil inputs)
  - [passes/](src/passes/) (optimisation passes)
  - [backend/](src/backend/) (Verilog file generation)

- [examples](examples/) (example files)

- [primitives/](primitives) (libraries)

  

## Makefile

`make build`: Downloads all dependencies and compile the source file.

`make install`: Installs caylx in the current folder.

`make [filename].futil`: Generate Futil program from Dahlia program. It requires to install [Dahlia](https://github.com/cucapra/dahlia) first.

`make [filename].v`: Generate Verilog RTL file from Futil program.

`make [filename].vcd`: Generate vcd file from Verilog RTL file. One can use ventilator to visualise it.

`make clean`: Deletes all generated files.

