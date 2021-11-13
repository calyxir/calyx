import csv
import subprocess
import statistics as st
from collections import defaultdict
from tabulate import tabulate

# Paths assumes you're running this script from the `futil` directory, i.e.
#   python3 evaluations/cidr-pldi-2022/process-data.py


def verify_interpreter_configuration():
    """
    Verifies the interpreter is in release mode and using
    the --no-verify flag.
    """
    configuration = subprocess.run(
        ["fud", "config", "stages.interpreter.exec"], capture_output=True
    )
    assert "release" in str(configuration.stdout), (
        "The interpreter should be in release mode. "
        + "To fix this, run `fud config stages.interpreter.exec .<PATH-TO-CALYX>/target/release/interp`."
    )

    configuration = subprocess.run(
        ["fud", "config", "stages.interpreter.flags"], capture_output=True
    )
    assert "--no-verify" in str(configuration.stdout), (
        "The interpreter should use the --no-verify flag. "
        + 'To fix this, run `fud config stages.interpreter.flags " --no-verify "`.'
    )


def process_data(dataset):
    """
    Runs the `evaluate-run.sh` script for each iteration of dataset.
    """
    for _, program, data, output in dataset:
        subprocess.run(
            ["evaluations/cidr-pldi-2022/evaluate-run.sh", program, data, output]
        )


def gather_data(dataset):
    """
    Returns a mapping from simulation name to the data, e.g.
    {
      "Dot Product" : {"verilog": [1.1, 2.1], "interpreter": [1.9, 2.2], ...}
    }
    """
    result = {}
    for name, _, _, output in dataset:
        with open(output) as file:
            # Mapping from stage to a list of durations.
            durations = defaultdict(list)
            for row in csv.reader(file, delimiter=","):
                # e.g. icarus-verilog,simulate,0.126
                assert len(row) == 3, "expected CSV row: <stage-name>,<step>,<time>"
                stage, _, time = row
                time = float(time)
                durations[stage].append(time)
            result[name] = durations
    return result


def table(name, data, tablefmt):
    """
    Prints a table in with the general layout as:
    <name> | stage1 | stage2 | ... | stageN
    mean   |
    median |
    stddev |
    """
    headers = [name]
    mean = ["mean"]
    median = ["median"]
    stddev = ["stddev"]
    for stage, times in sorted(data.items()):
        headers.append(stage)
        mean.append(st.mean(times))
        median.append(st.median(times))
        stddev.append(st.stdev(times))
    return tabulate([median, mean, stddev], headers, tablefmt)


def write_to_file(data, filename):
    """
    Appends `data` to `filename`. Assumes that
    data is a list.
    """
    assert isinstance(data, list)
    with open(filename, "a") as file:
        file.writelines("\n".join(data))


if __name__ == "__main__":
    verify_interpreter_configuration()

    # A list of datasets to evaluate simulation performance, in the form:
    # (<table-name>, <program-path>, <data-path>, <output-file-name>)
    datasets = [
        (
            "Dot Product",
            "examples/futil/dot-product.futil",
            "examples/dahlia/dot-product.fuse.data",
            "dot-product.csv",
        ),
    ]
    # Run the bash script for each dataset.
    # process_data(datasets)
    # Process the CSV.
    result = gather_data(datasets)

    # The formatting style of table.
    tablefmt = "latex"
    # Provide meaning to the data.
    tables = [table(name, data, tablefmt) for name, data in sorted(result.items())]
    write_to_file(tables, "results.txt")
