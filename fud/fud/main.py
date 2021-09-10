import argparse
import logging as log
from sys import exit
import importlib.util
from pathlib import Path

import toml

from . import check, errors, exec, utils
from .config import Configuration
from .registry import Registry
from .stages import (
    dahlia,
    futil,
    mrxl,
    relay,
    systolic,
    vcdump,
    verilator,
    vivado,
    xilinx,
    interpreter,
)


def register_stages(registry, cfg):
    """
    Register stages and command line flags required to generate the results.
    """
    # Dahlia
    registry.register(
        dahlia.DahliaStage(
            cfg, "futil", "-b futil --lower -l error", "Compile Dahlia to Calyx"
        )
    )
    registry.register(
        dahlia.DahliaStage(
            cfg,
            "vivado-hls",
            "--memory-interface ap_memory",
            "Compile Dahlia to Vivado C++",
        )
    )

    # MrXL
    registry.register(mrxl.MrXLStage(cfg))

    # Relay
    registry.register(relay.RelayStage(cfg))

    # Systolic Array
    registry.register(systolic.SystolicStage(cfg))

    # Calyx
    registry.register(
        futil.FutilStage(
            cfg,
            "verilog",
            "-b verilog",
            "Compile Calyx to Verilog instrumented for simulation",
        )
    )
    registry.register(
        futil.FutilStage(
            cfg,
            "synth-verilog",
            "-b verilog --synthesis -p external",
            "Compile Calyx to synthesizable Verilog ",
        )
    )
    registry.register(
        futil.FutilStage(
            cfg,
            "futil-lowered",
            "-b futil",
            "Compile Calyx to Calyx to remove all control and inline groups",
        )
    )
    registry.register(
        futil.FutilStage(
            cfg,
            "futil-noinline",
            "-b futil -d hole-inliner",
            "Compile Calyx to Calyx to remove all control and inline groups",
        )
    )
    registry.register(
        futil.FutilStage(
            cfg,
            "futil-externalize",
            "-b futil -p externalize",
            "Compile Calyx to Calyx to externalize all external memory primitives",
        )
    )
    registry.register(
        futil.FutilStage(
            cfg,
            "axi-wrapper",
            "-b xilinx",
            "Generate the AXI wrapper for Calyx",
        )
    )
    registry.register(
        futil.FutilStage(
            cfg,
            "xilinx-xml",
            "-b xilinx-xml",
            "Generate the XML metadata for Xilinx",
        )
    )
    registry.register(
        futil.FutilStage(
            cfg,
            "interpreter",
            "-p none",
            "Compile Calyx to Calyx",
        )
    )

    # Verilator
    registry.register(
        verilator.VerilatorStage(
            cfg, "vcd", "Generate a VCD file from Verilog simulation"
        )
    )
    registry.register(
        verilator.VerilatorStage(
            cfg, "dat", "Generate a JSON file with final state of all memories"
        )
    )

    # # Vivado / vivado hls
    registry.register(vivado.VivadoStage(cfg))
    registry.register(vivado.VivadoExtractStage(cfg))
    registry.register(vivado.VivadoHLSStage(cfg))
    registry.register(vivado.VivadoHLSExtractStage(cfg))

    # Vcdump
    registry.register(vcdump.VcdumpStage(cfg))

    # Xilinx
    registry.register(xilinx.XilinxStage(cfg))
    registry.register(xilinx.HwEmulationStage(cfg))
    registry.register(xilinx.HwExecutionStage(cfg))

    # Interpreter
    registry.register(interpreter.InterpreterStage(cfg, "", "", "Run the interpreter"))
    registry.register(
        interpreter.InterpreterStage.debugger(cfg, "", "", "Run the debugger")
    )
    # register external stages
    register_external_stages(cfg, registry)


def register_external_stages(cfg, registry):
    """
    Find external stages by looking for tables in the `external-stages` key
    and register them. An external stage must have a `location` key defined
    that points to a python module containing the external fud stages.
    """

    if ["external-stages"] in cfg:
        # search through external stages
        for stage in cfg[["external-stages"]]:
            # get file location of stage and ensure that it exists
            location = cfg["external-stages", stage, "location"]
            if not Path(location).exists():
                raise errors.InvalidExternalStage(
                    stage, f"No such file or directory: '{location}'"
                )

            # import the module from `location`
            spec = importlib.util.spec_from_file_location(stage, location)
            mod = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(mod)

            # check to make sure that module has `__STAGES__` defined.
            if not hasattr(mod, "__STAGES__"):
                raise errors.InvalidExternalStage(
                    stage, "The module doesn't have attribute: '__STAGES__'"
                )

            # register the discovered stages
            for stage_class in mod.__STAGES__:
                try:
                    registry.register(stage_class(cfg))
                except Exception as e:
                    raise errors.InvalidExternalStage(
                        stage,
                        "\n".join(
                            [
                                f"In {stage_class.__name__} from '{location}':",
                                "```",
                                str(e),
                                "```",
                            ]
                        ),
                    )


def display_config(args, cfg):
    if args.key is None:
        print(f"Configuration file location: {cfg.config_file}\n")
        cfg.display()
    else:
        path = args.key.split(".")
        if args.delete:
            del cfg[path]
            cfg.commit()
        elif args.value is None:
            # print out values
            res = cfg[path]
            if isinstance(res, dict):
                print(toml.dumps(res))
            else:
                print(res)
        else:
            # create configuration if it doesn't exist
            if path not in cfg:
                cfg[path] = args.value
            elif not isinstance(cfg[path], list):
                cfg[path] = args.value
            else:
                raise Exception(
                    "NYI: supporting updating lists. " + "Manually edit the file."
                )
            cfg.commit()


def main():
    """Builds the command line argument parser,
    parses the arguments, and returns the results."""

    parser = argparse.ArgumentParser(
        description="Driver to execute Calyx and supporting toolchains"
    )
    # Name of the subparser stored in command
    subparsers = parser.add_subparsers()

    config_run(
        subparsers.add_parser(
            "exec",
            help="Execute one of the Calyx-related tools",
            description="Execute one of the Calyx-related tools",
            aliases=["e", "ex"],
        )
    )
    config_config(
        subparsers.add_parser(
            "config",
            help="Configure environment variables for the driver",
            description="Configure environment variables for the driver",
            aliases=["c"],
        )
    )
    config_info(
        subparsers.add_parser(
            "info",
            help="Show information about execution stages",
            description="Show information about execution stages",
            aliases=["i"],
        )
    )
    config_check(
        subparsers.add_parser(
            "check",
            help="Check to make sure configuration is valid.",
            description="Check to make sure configuration is valid.",
        )
    )
    config_register(
        subparsers.add_parser(
            "register",
            help="Register an external stage.",
            description="Register an external stage.",
        )
    )

    args = parser.parse_args()
    # Setup logging
    utils.logging_setup(args)

    if "command" not in args:
        parser.print_help()
        exit(-1)

    try:
        cfg = Configuration()

        # update the stages config with arguments provided via cmdline
        if "dynamic_config" in args and args.dynamic_config is not None:
            for key, value in args.dynamic_config:
                cfg[["stages"] + key.split(".")] = value

        # Build the registry if stage information is going to be used.
        if args.command in ("exec", "info"):
            cfg.REGISTRY = Registry(cfg)
            register_stages(cfg.REGISTRY, cfg)

        if args.command == "exec":
            exec.run_fud(args, cfg)
        elif args.command == "info":
            print(cfg.REGISTRY)
        elif args.command == "config":
            display_config(args, cfg)
        elif args.command == "check":
            check.check(cfg)
        elif args.command == "register":
            cfg.setup_external_stage(args)

    except errors.FudError as e:
        log.error(e)
        exit(-1)


# this is very silly, but needed because argparse doesn't like arguments which are just flags
def trim_space(in_str):
    return in_str.strip()


def config_run(parser):
    parser.add_argument("--from", dest="source", help="Name of the start stage")
    parser.add_argument("--to", dest="dest", help="Name of the final stage")
    parser.add_argument(
        "-o", dest="output_file", help="Name of the outpfule file (default: STDOUT)"
    )
    parser.add_argument(
        "-s",
        help="Override configuration key-value pairs for this run",
        nargs=2,
        metavar=("key", "value"),
        dest="dynamic_config",
        action="append",
        type=trim_space,
    )
    parser.add_argument(
        "-n",
        "--dry-run",
        action="store_true",
        dest="dry_run",
        help="Show the execution stages and exit",
    )
    parser.add_argument(
        "-v", "--verbose", action="count", default=0, help="Enable verbose logging"
    )
    parser.add_argument("-q", "--quiet", action="store_true")
    parser.add_argument("input_file", help="Path to the input file", nargs="?")
    parser.set_defaults(command="exec")


def config_config(parser):
    parser.add_argument("key", help="The key to perform an action on.", nargs="?")
    parser.add_argument("value", help="The value to write.", nargs="?")
    parser.add_argument(
        "-d", "--delete", help="Remove key from config.", action="store_true"
    )
    parser.set_defaults(command="config")


def config_info(parser):
    parser.set_defaults(command="info")


def config_check(parser):
    parser.set_defaults(command="check")


def config_register(parser):
    parser.add_argument("name", help="The name of the external stage to be registered.")
    parser.add_argument(
        "-p --path", help="The path of the stage to be registered.", dest="path"
    )
    parser.add_argument(
        "-d --delete",
        help="Removes an external registered stage.",
        dest="delete",
        action="store_true",
    )
    parser.set_defaults(command="register")
