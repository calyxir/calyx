#! python3

from argparse import ArgumentParser
import random
import shutil
from pathlib import Path
import difflib
import sys
import subprocess
import signal

WORKDIR_NAME = ".fud2_datarace_baseline"


def run(
    test_file: Path, policy: str, data_file: Path | None, entangle: list[str]
) -> str:
    match policy:
        case "seq":
            policy_str = "random_serialized"
        case "random":
            policy_str = "random"

    entangle_str = " ".join((f"--entangle '{item}'" for item in entangle))

    arg_list = [
        "fud2",
        "--dir",
        WORKDIR_NAME,
        "--to",
        "dat",
        "--through",
        "cider",
        "-s",
        f"cider.flags=--dump-registers --policy {policy_str} {entangle_str}",
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

    with open(Path(WORKDIR_NAME) / Path("dat_1.json")) as w:
        out = "".join(w.readlines())

    return out


def detect_data_race(
    test_file: Path,
    policy: str,
    data_file: Path | None,
    entangle: list[str],
):
    first = run(test_file, policy, data_file, entangle)

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
            sys.exit(i + 1)
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
    parser.add_argument("--entangle", action="append", default=[])

    random_suffix = random.randint(0, 1000000000)
    global WORKDIR_NAME
    WORKDIR_NAME = f"{WORKDIR_NAME}_{random_suffix}"

    args = parser.parse_args()

    signal.signal(signal.SIGALRM, lambda *x: handler(args.timeout, *x))
    signal.alarm(args.timeout)

    detect_data_race(args.file, args.mode, args.data, args.entangle)


if __name__ == "__main__":
    main()
