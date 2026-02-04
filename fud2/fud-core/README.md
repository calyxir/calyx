# fud-core
Before reading this, it's probably worth getting some background on the high level design [here](../../docs/running-calyx/fud2/index.md).

## General Execution Flow
The common execution flow of fud2 is to start by parsing scripts which define ops and states (done in `src/script`). This creates a driver, encapsulating the hypergraph of ops and states. This is then eventually passed to `cli_ext` in `src/cli.rs`. Here, a request is created and sent to the driver. This request represents a user's command line query, their inputs and outputs and throughs. This request is then run by the driver which uses one of the planners (`src/exec/plan`) to create a plan. These plans are sometimes called "flang programs" or simply an "ir" in the code base. This plan is then lowered to ninja and executed by code in `run.rs`.
