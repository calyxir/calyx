from pathlib import Path
from termcolor import colored, cprint
import shutil
import subprocess
from packaging import version
import sys
from fud import config

# Dictionary that defines how to check the version for different tools.
# Maps names of stage to the command needed to check the tool.
VERSIONS = {
    "dahlia": {
        "flag": "--version",
        "extract": lambda out: out,
        "version": "dirty",
        "compare": "status_is_not",
        "help": "Dahlia binary built using uncommitted changes. "
        + "Please commit changes to the Dahlia compiler and rebuild it.",
    },
    "verilog": {
        "flag": "--version",
        "extract": lambda out: out.split(" ")[1],
        "version": "5.002",
        "compare": ">=",
        "help": "Try building from source: "
        + "https://www.veripool.org/projects/verilator/wiki/Installing",
    },
    "vcd": {
        "flag": "--version",
        "extract": lambda out: out.split(" ")[1],
        "version": "0.1.2",
        "compare": ">=",
        "help": "Run `cargo install vcdump` to update.",
    },
    "vivado": {
        "flag": "-version",
        "extract": lambda out: out.split(" ")[1],
        "version": "v2019.2",
        "compare": "==",
    },
    "vivado_hls": {
        "flag": "-version",
        "extract": lambda out: out.split(" ")[10],
        "version": "v2019.2",
        "compare": "==",
    },
    "icarus-verilog": {
        "flag": "-V",
        "extract": lambda out: out.split(" ")[3],
        "version": "11.0",
        "compare": ">=",
    },
}


def version_compare(cmp_str, installed, required):
    """
    Given a `cmp_str`, call the related comparison function on
    `installed` {cmp op} `required`.
    """
    if cmp_str == ">=":
        return version.parse(installed) >= version.parse(required)
    if cmp_str == "==":
        return version.parse(installed) == version.parse(required)
    if cmp_str == "<=":
        return version.parse(installed) <= version.parse(required)
    if cmp_str == "status_is_not":
        return required not in installed

    raise Exception(f"Unknown compare string: {cmp_str}")


def check_version(name, exec_path):
    """
    Check the version for the stage: `name`.
    """
    try:
        if name in VERSIONS:
            info = VERSIONS[name]
            proc = subprocess.run(
                [exec_path, info["flag"]],
                stdout=subprocess.PIPE,
                check=False,
            )
            install = info["extract"](proc.stdout.decode("UTF-8")).strip()
            if version_compare(info["compare"], install, info["version"]):
                cprint(" ✔", "green", end=" ")
                print("Found version", end=" ")
                cprint(f"{install}", "yellow", end=" ")
                print(f"({info['compare']} ", end="")
                cprint(f"{info['version']}", "yellow", end="")
                print(")", end="")
                print(".")
                return True

            cprint(" ✖", "red", end=" ")
            print("Found version", end=" ")
            cprint(f"{install},", "yellow", end=" ")
            print(f"but need version {info['compare']} ", end="")
            cprint(f"{info['version']}", "yellow", end="")
            print(".")
            if "help" in info:
                cprint(f"   {info['help']}")
            return False

        return True
    except OSError as e:
        cprint(" ✖", "red", end=" ")
        print(f"Error during version check: {e}")


def check(args, cfg):
    """
    Check the entire configuration `cfg`.
    """
    cfg.launch_wizard()

    # check global
    futil_root = Path(cfg["global", config.ROOT])
    futil_str = colored(str(futil_root), "yellow")
    cprint("global:", attrs=["bold"])
    if futil_root.exists():
        cprint(" ✔", "green", end=" ")
        print(f"{futil_str} exists.")
    else:
        cprint(" ✖", "red", end=" ")
        print(f"{futil_str} doesn't exist.")
    print()

    uninstalled = []
    wrong_version = []
    stages = set(args.stages) if args.stages else None

    # check executables in stages
    for name, stage in cfg["stages"].items():
        if stages is not None:
            if name not in stages:
                continue
            else:
                stages.remove(name)

        if "exec" in stage:
            cprint(f"stages.{name}.exec:", attrs=["bold"])
            exec_path = shutil.which(stage["exec"])
            exec_name = colored(stage["exec"], "yellow")
            if stage["exec"].startswith("cargo run"):
                cprint(" ✔", "green", end=" ")
            elif exec_path is not None:
                cprint(" ✔", "green", end=" ")
                print(f"{exec_name} installed.")
                # check if path is absolute or relative
                if not Path(exec_path).is_absolute():
                    print(
                        f"   {exec_name} is a relative path and "
                        + "will not work from every directory."
                    )
                # check version
                if not check_version(name, exec_path):
                    wrong_version.append(name)
            else:
                uninstalled.append(name)
                cprint(" ✖", "red", end=" ")
                print(f"{exec_name} not installed.")
            print()

    if len(uninstalled) > 0:
        bad_stages = colored(", ".join(uninstalled), "red")
        verb = "were" if len(uninstalled) > 1 else "was"
        print(f"{bad_stages} {verb} not installed correctly.")
        print(
            "Configuration instructions: "
            + "https://docs.calyxir.org/fud/#configuration"
        )

    if len(wrong_version) > 0:
        # add line break if we printed out uninstalled
        if len(uninstalled) > 0:
            print()
        bad_stages = colored(", ".join(wrong_version), "red")
        verb = "versions" if len(wrong_version) > 1 else "version"
        print(f"Incorrect {verb} for {bad_stages}.")
        print("Stages may fail or not execute correctly.")

    if stages:
        bad_stages = colored(", ".join(stages), "red")
        print(f"Stage(s) not found in configuration: {bad_stages}")

    # exit with -1 if something went wrong
    if len(uninstalled) > 0 or len(wrong_version) > 0:
        sys.exit(-1)
