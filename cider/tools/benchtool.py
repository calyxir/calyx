#! python3

# A simple script runner designed to run cider with hyperfine on the same calyx program
# across multiple different revisions

from argparse import ArgumentParser
from pathlib import Path
import shutil
import subprocess
from typing import List
import re

WORKDIR_NAME = ".fud2_benchtool"


def check_dependency(dependency: str):
    proc = subprocess.run(
        ["which", dependency], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
    )
    if proc.returncode != 0:
        raise RuntimeError(f"{dependency} is not installed or not available")


def check_dependencies():
    check_dependency("hyperfine")
    check_dependency("fud2")


def setup_sim(fud2_command: str, timeout: int):
    assert "fud2" in fud2_command, "fud2 command is malformed"
    assert "--keep" not in fud2_command, "fud2 command may not contain the --keep flag"
    assert "--dir" not in fud2_command, "fud2 command may not contain the --dir flag"

    comm = fud2_command + f" --dir {WORKDIR_NAME}"
    try:
        subprocess.run(
            comm, timeout=timeout, shell=True, capture_output=True
        ).check_returncode()
    except subprocess.TimeoutExpired:
        pass


def extract_calyx_path() -> Path:
    with open(Path(WORKDIR_NAME) / "build.ninja") as f:
        for line in f.readlines():
            if "calyx-base =" in line:
                base = line.removeprefix("calyx-base =").strip()
                return Path(base)
    raise RuntimeError("unable to computer cider path from ninja file")


def run_hyperfine(command: str, calyx_path: Path, revisions: List[str]):
    # this is a bad regex, do not look at it
    flag_regex = re.compile(r"cider\.flags=((\".*\")|('.*'))")
    flags = flag_regex.search(command)
    if flags:
        # add one for the initial quote
        prefix_len = len("cider.flags=") + 1
        flags = " " + flags.group()[prefix_len:-1]
    else:
        flags = ""

    data_str = ""
    if (Path(WORKDIR_NAME) / "data.dump").exists():
        data_str = " --data data.dump"

    base_dir = calyx_path

    original_revision = subprocess.run(
        ["jj", "log", "-r", "@", "-G"], capture_output=True, text=True
    ).stdout.split()[0]

    argument_list = [
        "hyperfine",
        "--warmup",
        "1",
        "--cleanup",
        f"jj e {original_revision}",
    ]
    for revision in revisions:
        argument_list.append("--prepare")
        # this takes advantage of the fact that opening a new revision for the same
        # parent on an empty revision reuses that revision so the compilation will only
        # happen once
        argument_list.append(f"jj new {revision}; cargo build --release")
        argument_list.extend(["-n", revision])
        argument_list.append(
            f"{calyx_path / 'target/release/cider'} -l {base_dir} pseudo_cider{data_str}{flags}"
        )

    subprocess.run(argument_list, cwd=WORKDIR_NAME).check_returncode()


def cleanup():
    shutil.rmtree(Path(WORKDIR_NAME))


def main():
    parser = ArgumentParser()
    parser.add_argument("command", type=str)
    parser.add_argument("--revisions", nargs="+")
    parser.add_argument(
        "-t",
        "--timeout",
        type=int,
        default=3,
        help="the amount of time fud2 is allowed to run when setting up the simulation",
    )

    args = parser.parse_args()

    check_dependencies()
    setup_sim(args.command, args.timeout)
    cider_path = extract_calyx_path()

    run_hyperfine(args.command, cider_path, args.revisions)
    cleanup()


if __name__ == "__main__":
    main()
