## Steps

These are the steps listed for running the benchmarks. With each step, a link is provided with
extra information in case something goes wrong during installation. To verify `fud` stages are
installed correctly, you may also call `fud check`.

0. Install [Rust](https://doc.rust-lang.org/cargo/getting-started/installation.html)

```bash
# On Mac and Linux
curl https://sh.rustup.rs -sSf | sh
```

1. Install [Calyx](https://capra.cs.cornell.edu/docs/calyx/intro.html#compiler-installation)

```bash
git clone https://github.com/cucapra/calyx.git && cargo build
```

2. Install [`fud`](https://capra.cs.cornell.edu/docs/calyx/intro.html#installing-the-command-line-driver)

```bash
pip3 install flit && flit -f fud/pyproject.toml install -s
fud config global.futil_directory <full path to Calyx repository>
```

3. Install [Verilator](https://capra.cs.cornell.edu/docs/calyx/intro.html#simulating-with-verilator) (necessary for Verilog benchmarks)

```bash
# On Mac:
brew install verilator

# On Linux
git clone https://github.com/verilator/verilator
cd verilator && git pull && git checkout master && autoconf
./configure && make && sudo make install
```

4. Install [Calyx Interpreter](https://capra.cs.cornell.edu/docs/calyx/interpreter.html#interpreting-via-fud) (necessary for interpreter benchmarks)

```bash
# From the futil repository, build in release mode.
cd interp && cargo build --release

# Additionally, add the `--no-verify` flag.
fud config stages.interpreter.flags \"--no-verify\"
```

5. Install [Icarus-Verilog](https://capra.cs.cornell.edu/docs/calyx/fud/index.html#icarus-verilog) (necessary for Icarus-Verilog benchmarks)

```bash
fud register icarus-verilog -p fud/icarus/icarus.py

# Set Verilog to high priority.
fud c stages.verilog.priority 1
```

6. Install [Dahlia frontend](https://capra.cs.cornell.edu/docs/calyx/fud/index.html#dahlia-frontend) (necessary for Polybench benchmarks)

```bash
git clone https://github.com/cucapra/dahlia.git && cd dahlia && sbt install
sbt assembly && chmod +x ./fuse

fud config stages.dahlia.exec <full path to dahlia repo>/fuse
```

7. Install [NTT](https://capra.cs.cornell.edu/docs/calyx/frontends/ntt.html#installation) (necessary for NTT benchmarks)
```bash
# From the futil repository
cd calyx-py && flit install -s && pip3 install prettytable numpy
fud register ntt -p frontends/ntt-pipeline/fud/ntt.py && fud check
```

8. Run the script

```bash
# From the futil repository
chmod u+x evaluations/cidr-pldi-2022/scripts/evaluate.sh
chmod u+x evaluations/cidr-pldi-2022/scripts/evaluate-fully-lowered.sh
mkdir evaluations/cidr-pldi-2022/individual-results && mkdir evaluations/cidr-pldi-2022/statistics   
python3 evaluations/cidr-pldi-2022/process-data.py
```

This should result in 3 files in `evaluations/cidr-pldi-2022/statistics` (as well as individual run results in `/individual-results`):
- `simulation-fully-lowered-results.csv`: Simulation statistics for the interpreter after fully lowering the Calyx program. 
- `simulation-results.csv`: Simulation statistics for interpreter, Verilog, and Icarus-Verilog.
- `compilation-results.csv` Compilation statistics for Verilog and Icarus-Verilog.