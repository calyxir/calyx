# FUD: Calyx Driver
This is the Calyx driver. It is a tool that automates the process
of calling Calyx frontends, the Calyx compiler, and any backends that may
be needed to simulate/execute a program.

The current documentation for fud lives [here](https://docs.calyxir.org/fud/index.html).

## Contributing
We use `black` as an automatic formatter and `flake8` to lint `fud`s code. Before merging
any changes to fud, please address any warnings from `flake8 .` and run `black .` to format code.

You can install these tools with:
```
pip3 install flake8 black
```
