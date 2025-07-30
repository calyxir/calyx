#! python3

from argparse import ArgumentParser
import shutil
from pathlib import Path
import difflib
import sys
import subprocess
import signal

WORKDIR_NAME = ".fud2_datarace_baseline"


def run(test_file: Path, policy: str, data_file: Path | None = None) -> str:
    match policy:
        case "seq":
            policy_str = "random_serialized"
        case "random":
            policy_str = "random"

    arg_list = [
        "fud2",
        "--dir",
        WORKDIR_NAME,
        "--to",
        "dat",
        "--through",
        "cider",
        "-s",
        f"cider.flags=--dump-registers --policy {policy_str}",
        test_file,
    ]
    if data_file is not None:
        arg_list += ["-s", f"sim.data={data_file}"]

    process = subprocess.run(
        arg_list,
        capture_output=True,
        text=True,
    )

    if process.returncode != 0:
        print(process.stderr, file=sys.stderr)
        sys.stderr.flush()
        process.check_returncode()

    return process.stdout


def rerun() -> str:
    subprocess.run(["rm", "interp_out.dump"], cwd=WORKDIR_NAME)

    process = subprocess.run(
        ["ninja"], cwd=WORKDIR_NAME, text=True, capture_output=True
    )

    if process.returncode != 0:
        print(process.stderr, file=sys.stderr)
        sys.stderr.flush()
        process.check_returncode()

    with open(Path(WORKDIR_NAME) / Path("_to_stdout_dat.json")) as w:
        out = "".join(w.readlines())

    return out


def detect_data_race(test_file: Path, policy: str, data_file: Path | None = None):
    first = run(test_file, policy, data_file)

    i = 0
    while True:
        current = rerun()
        if first != current:
            different_lines = difflib.unified_diff(
                first.splitlines(True),
                current.splitlines(True),
                fromfile="Original",
                tofile=f"Execution #{i + 1}",
            )
            print("Data Race detected\n", file=sys.stderr)
            sys.stderr.writelines(different_lines)
            sys.stderr.flush()

            cleanup()
            sys.exit(101)
        else:
            i += 1


def cleanup():
    shutil.rmtree(Path(WORKDIR_NAME))


def handler(timeout, *args):
    print(f"Timeout reached. No difference discovered after {timeout} seconds")
    cleanup()
    sys.exit(0)


def main():
    parser = ArgumentParser()
    parser.add_argument("file", type=Path)
    parser.add_argument("--data", type=Path, default=None)
    parser.add_argument("-m", "--mode", choices=["seq", "random"], default="seq")
    parser.add_argument("-t", "--timeout", type=int, default=30)

    args = parser.parse_args()

    signal.signal(signal.SIGALRM, lambda *x: handler(args.timeout, *x))
    signal.alarm(args.timeout)

    detect_data_race(args.file, args.mode, args.data)


if __name__ == "__main__":
    main()
