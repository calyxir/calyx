import csv
import subprocess
import time
import statistics as st
from collections import defaultdict

# Paths assumes you're running this script from the `futil` directory, i.e.
#   python3 evaluations/cidr-pldi-2022/process-data.py


def verify_interpreter_configuration():
    """
    Verifies the interpreter is in release mode and using
    the --no-verify flag.
    """

    def command_has_value(command, value, error):
        """
        Verifies that the stdout of this `command` has `value` in it.
        """
        process = subprocess.run(command, capture_output=True)
        assert value in str(process.stdout), error

    command_has_value(
        ["fud", "config", "stages.interpreter.exec"],
        "release",
        "The interpreter should be in release mode. "
        + "To fix this, run `fud config stages.interpreter.exec .<PATH-TO-CALYX>/target/release/interp`.",
    )

    command_has_value(
        ["fud", "config", "stages.interpreter.flags"],
        "--no-verify",
        "The interpreter should use the --no-verify flag. "
        + 'To fix this, run `fud config stages.interpreter.flags " --no-verify "`.',
    )


def get_csv_filename(name, lowered):
    """
    Uses the simulation name to produce the CSV file name, e.g. `Dot Product`
     -> `evaluations/cidr-pldi-2022/individual-results/Dot_Product.csv`

     We give slightly differe names to fully lowered Calyx programs, since Fud can't
     differentiate the two:
        `evaluations/cidr-pldi-2022/individual-results/Dot_Product-Lowered.csv`
    """
    return (
        "evaluations/cidr-pldi-2022/individual-results/"
        + name.replace(" ", "_")
        + ("-Lowered" if lowered else "")
        + ".csv"
    )


def process_data(dataset, is_fully_lowered, path, script):
    """
    Runs the script for each iteration of dataset. `is_fully_lowered` is
    just used to distinguish file names.
    """
    for name, program in dataset:
        subprocess.run(
            [
                script,
                path + program,
                # Assumes that the data is the same path with `.data` appended.
                path + program + ".data",
                get_csv_filename(name, is_fully_lowered),
                "10",  # Number of simulations per program.
            ]
        )


def gather_data(dataset, is_fully_lowered):
    """
    Returns two mappings from simulation name to the data for both simulation
    and compilation times, e.g.
    {
      "Dot Product" : {"verilog": [1.1, 2.1], "interpreter": [1.9, 2.2], ...}
    }
    """
    simulations = {}
    compilations = {}
    for name, _ in dataset:
        # Just use the simulation name, e.g. Dot Product -> Dot_Product.csv
        with open(get_csv_filename(name, is_fully_lowered)) as file:
            # Mapping from stage to a list of durations.
            simtimes = defaultdict(list)
            comptimes = defaultdict(list)
            for row in csv.reader(file, delimiter=","):
                # e.g. icarus-verilog,simulate,0.126
                assert len(row) == 3, "expected CSV row: <stage-name>,<step>,<time>"
                stage, step, time = row
                time = float(time)
                if "compile" not in step:
                    # This is a simulation step.
                    simtimes[stage].append(time)
                else:
                    comptimes[stage].append(time)
            simulations[name] = simtimes
            compilations[name] = comptimes

    return simulations, compilations


def write_csv_results(type, results):
    """
    Writes a CSV file with the format:
    `type,stage,mean,median,stddev`

    to `evaluations/cidr-pldi-2022/statistics/<type>-results.csv`.
    """
    with open(
        f"evaluations/cidr-pldi-2022/statistics/{type}-results.csv", "a", newline=""
    ) as file:
        writer = csv.writer(file, delimiter=",")
        writer.writerow([type, "stage", "mean", "median", "stddev"])
        for name, data in results.items():
            for stage, times in data.items():
                mean = round(st.mean(times), 3)
                median = round(st.median(times), 3)
                stddev = round(st.stdev(times), 3)
                writer.writerow([name, stage, mean, median, stddev])


def write_to_file(data, filename):
    """
    Appends `data` to `filename`. Assumes that
    data is a list.
    """
    assert isinstance(data, list)
    with open(filename, "a") as file:
        file.writelines("\n".join(data))


def run(data, script):
    """
    Runs the simulation and data processing on the datasets.
    """
    # Run a different script for fully lowered Calyx. These are separated since Fud
    # has no way to dinstinguish profiling stage names based on previous stages.
    is_fully_lowered = "fully-lowered" in script

    # Run the bash script for each dataset.
    process_data(
        data,
        is_fully_lowered,
        path="evaluations/cidr-pldi-2022/benchmarks/",
        script=f"evaluations/cidr-pldi-2022/scripts/{script}",
    )
    # Process the CSV.
    simulations, compilations = gather_data(data, is_fully_lowered)
    # Provide meaning to the data.
    if is_fully_lowered:
        write_csv_results("simulation-fully-lowered", simulations)
    else:
        # No compilation for this, since we only run interpreter simulation for the fully-lowered script.
        write_csv_results("compilation", compilations)
        write_csv_results("simulation", simulations)


if __name__ == "__main__":
    verify_interpreter_configuration()

    # A list of datasets to evaluate simulation performance, in the form:
    # (<table-name>, <program-path>). We just assume the data is at the same
    # path with `.data` appended. The path is relative to:
    #     futil/evaluations/cidr-pldi-2022/benchmarks
    datasets = [
        (
            "NTT 32",
            "ntt-32.futil",
        ),
        (
            "NTT 64",
            "ntt-64.futil",
        ),
        (
            "TCAM 32",
            "tcam-32.futil",
        ),
        (
            "TCAM 64",
            "tcam-64.futil",
        ),
        # Polybench
        (
            "Linear Algebra 2MM",
            "polybench/linear-algebra-2mm.fuse",
        ),
        (
            "Linear Algebra 3MM",
            "polybench/linear-algebra-3mm.fuse",
        ),
        (
            "Linear Algebra ATAX",
            "polybench/linear-algebra-atax.fuse",
        ),
        (
            "Linear Algebra BICG",
            "polybench/linear-algebra-bicg.fuse",
        ),
        (
            "Linear Algebra DOITGEN",
            "polybench/linear-algebra-doitgen.fuse",
        ),
        (
            "Linear Algebra DURBIN",
            "polybench/linear-algebra-durbin.fuse",
        ),
        (
            "Linear Algebra GEMM",
            "polybench/linear-algebra-gemm.fuse",
        ),
        (
            "Linear Algebra GEMVER",
            "polybench/linear-algebra-gemver.fuse",
        ),
        (
            "Linear Algebra GESUMMV",
            "polybench/linear-algebra-gesummv.fuse",
        ),
        (
            "Linear Algebra LU",
            "polybench/linear-algebra-lu.fuse",
        ),
        (
            "Linear Algebra LUDCMP",
            "polybench/linear-algebra-ludcmp.fuse",
        ),
        (
            "Linear Algebra MVT",
            "polybench/linear-algebra-mvt.fuse",
        ),
        (
            "Linear Algebra SYMM",
            "polybench/linear-algebra-symm.fuse",
        ),
        (
            "Linear Algebra SYR2K",
            "polybench/linear-algebra-syr2k.fuse",
        ),
        (
            "Linear Algebra SYRK",
            "polybench/linear-algebra-syrk.fuse",
        ),
        (
            "Linear Algebra TRISOLV",
            "polybench/linear-algebra-trisolv.fuse",
        ),
        (
            "Linear Algebra TRMM",
            "polybench/linear-algebra-trmm.fuse",
        ),
    ]

    print("Beginning benchmarks...")
    begin = time.time()
    # Run normal benchmarks on interpreter, Verilog, Icarus-Verilog.
    run(datasets, "evaluate.sh")
    # Run benchmarks on fully lowered Calyx through the interpreter.
    run(datasets, "evaluate-fully-lowered.sh")

    duration = (begin - time.time()) / 60.0
    print(f"Benchmarks took approximately: {int(duration)} minutes.")
