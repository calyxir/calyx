import argparse
import logging as log
from sys import exit
import os

import toml

from . import check, errors, exec, utils, external
from .config import Configuration
from .registry import Registry
from .stages import (
    dahlia,
    futil,
    interpreter,
    relay,
    systolic,
    vcdump,
    jq,
    verilator,
    vivado,
    xilinx,
)


def register_stages(registry):
    """
    Register stages and command line flags required to generate the results.
    """
    # Dahlia
    registry.register(
        dahlia.DahliaStage(
            "calyx", "-b calyx --lower -l error", "Compile Dahlia to Calyx"
        )
    )
    registry.register(
        dahlia.DahliaStage(
            "vivado-hls",
            "--memory-interface ap_memory",
            "Compile Dahlia to Vivado C++",
        )
    )

    # Relay
    registry.register(relay.RelayStage())
    # Systolic Array
    registry.register(systolic.SystolicStage())
    # Calyx
    registry.register(
        futil.CalyxStage(
            "verilog",
            "-b verilog",
            "Compile Calyx to Verilog instrumented for simulation",
        )
    )
    registry.register(
        futil.CalyxStage(
            "mlir",
            "-b mlir -p well-formed -p lower-guards",
            "Compile Calyx to MLIR",
        )
    )
    registry.register(
        futil.CalyxStage(
            "synth-verilog",
            "-b verilog --synthesis -p external --disable-verify",
            "Compile Calyx to synthesizable Verilog",
        )
    )
    registry.register(
        futil.CalyxStage(
            "calyx-lowered",
            "-b calyx",
            "Compile Calyx to Calyx to remove all control and inline groups",
        )
    )
    registry.register(
        futil.CalyxStage(
            "calyx-noinline",
            "-b calyx -d hole-inliner",
            "Compile Calyx to Calyx to remove all control and inline groups",
        )
    )
    registry.register(
        futil.CalyxStage(
            "calyx-externalize",
            "-b calyx -p externalize",
            "Compile Calyx to Calyx to externalize all external memory primitives",
        )
    )
    registry.register(
        futil.CalyxStage(
            "axi-wrapper",
            "-b xilinx",
            "Generate the AXI wrapper for Calyx",
        )
    )
    registry.register(
        futil.CalyxStage(
            "xilinx-xml",
            "-b xilinx-xml",
            "Generate the XML metadata for Xilinx",
        )
    )
    registry.register(
        futil.CalyxStage(
            "interpreter",
            "-p none",
            "Compile Calyx for interpretation with CIDR",
        )
    )
    registry.register(
        futil.CalyxStage(
            "resources",
            "-b resources",
            "Generate a CSV that estimates a Calyx program's resource usage",
        )
    )

    # Data conversion
    registry.register(verilator.JsonToDat())
    registry.register(verilator.DatToJson())

    # Verilator
    registry.register(
        verilator.VerilatorStage("vcd", "Generate a VCD file from Verilog simulation")
    )
    registry.register(
        verilator.VerilatorStage(
            "dat", "Generate a JSON file with final state of all memories"
        )
    )

    # # Vivado / vivado hls
    registry.register(vivado.VivadoStage())
    registry.register(vivado.VivadoExtractStage())
    registry.register(vivado.VivadoHLSStage())
    registry.register(vivado.VivadoHLSExtractStage())
    registry.register(vivado.VivadoHLSPlaceAndRouteStage())
    registry.register(vivado.VivadoHLSPlaceAndRouteExtractStage())

    # Vcdump
    registry.register(vcdump.VcdumpStage())

    # Jq
    registry.register(jq.JqStage("vcd_json"))
    registry.register(jq.JqStage("dat"))
    registry.register(jq.JqStage("interpreter-out"))

    # Xilinx
    registry.register(xilinx.XilinxStage())
    registry.register(xilinx.HwExecutionStage())

    # Interpreter
    registry.register(interpreter.InterpreterStage("", "", "Run the interpreter"))
    registry.register(interpreter.InterpreterStage.debugger("", "", "Run the debugger"))
    registry.register(interpreter.InterpreterStage.data_converter())


def register_external_stages(cfg, registry):
    """
    Find external stages and register them.
    An external stage at least has the fields `location` and `external`.
    Key values are filled in this order:
        1. Dynamic keys using -s <key> <value>
        2. Keys defined in the configuration
    """

    # No externals to load.
    if not ["externals"] in cfg:
        return

    for ext, location in cfg[["externals"]].items():
        mod = external.validate_external_stage(ext, cfg)

        # register the discovered stages
        for stage_class in mod.__STAGES__:
            try:
                registry.register(stage_class())
            except Exception as e:
                raise errors.InvalidExternalStage(
                    ext,
                    "\n".join(
                        [
                            f"In {stage_class.name} from '{location}':",
                            "```",
                            str(e),
                            "```",
                        ]
                    ),
                ) from e


def display_or_edit_config(args, cfg):
    """Print out the value in the config or update it"""
    # If no key is specified, print out the entire config
    if args.key is None:
        print(f"Configuration file location: {cfg.config_file}\n")
        cfg.display()
        return

    # Construct a path from the key specification
    path = args.key.split(".")

    # Delete the key if --delete is specified
    if args.delete:
        del cfg[path]
        cfg.commit()
        return

    # If no value is specified, print out the value at the path
    if args.value is None:
        # print out values
        res = cfg[path]
        if isinstance(res, dict):
            print(toml.dumps(res))
        else:
            print(res)
        return

    # Update the path with the provided value
    val = int(args.value) if args.value.isdigit() else args.value
    # create configuration if it doesn't exist
    if path not in cfg:
        # Don't create a new field unless --create is specified
        if not args.create:
            raise errors.FudError(
                f"Path `{'.'.join(path)}' does not exist. Provide the --create flag if "
                " you meant to create a new field instead of updating an existing one."
            )
        cfg[path] = val
    elif not isinstance(cfg[path], list):
        cfg[path] = val
    else:
        raise errors.FudError(
            "Cannot update a field. Use `fud c --edit` to"
            " manually edit the configuration"
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

    run_parser = config_run(
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
            help="Register external stages.",
            description="Register external stages.",
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

        # Only allow either config_file or dynamic configurations
        if ("stage_dynamic_config" in args and args.stage_dynamic_config) and (
            "config_file" in args and args.config_file
        ):
            run_parser.error(
                "Please provide either a configuration file or"
                + " dynamic configurations",
            )

        # update the stages config with arguments provided via cmdline
        if "stage_dynamic_config" in args and args.stage_dynamic_config is not None:
            for key, value in args.stage_dynamic_config:
                cfg[["stages"] + key.split(".")] = value

        if "config_file" in args and args.config_file is not None:
            # Parse the TOML file
            override = toml.load(args.config_file)
            for key, value in override.items():
                if key != "stages":
                    log.warn(
                        f"Ignoring key `{key}' in config file."
                        + " Only 'stages' is allowed as a top-level key."
                    )
            # Hide all unused keys
            override = override["stages"]
            cfg.update_all({"stages": override})

        # Build the registry if stage information is going to be used.
        if args.command in ("exec", "info"):
            cfg.registry = Registry(cfg)
            register_stages(cfg.registry)
            register_external_stages(cfg, cfg.registry)

        if args.command == "exec":
            if not (args.input_file or args.source):
                run_parser.error(
                    "Please provide either an input file or a --from option"
                )
            if not (args.output_file or args.dest):
                run_parser.error(
                    "Please provide either an output file or a --to option"
                )

            exec.run_fud_from_args(args, cfg)
        elif args.command == "info":
            print(cfg.registry)
        elif args.command == "config":
            if args.edit:
                os.system(f"{os.getenv('EDITOR')} '{cfg.config_file}'")
            elif args.remove:
                log.info(f"Removing {cfg.config_file}")
                os.remove(cfg.config_file)
            else:
                display_or_edit_config(args, cfg)
        elif args.command == "check":
            check.check(args, cfg)
        elif args.command == "register":
            cfg.setup_external_stage(args)

    except errors.FudError as e:
        log.error(e)
        exit(-1)


def config_run(parser):
    parser.add_argument(
        "-pr",
        "--dump_prof",
        nargs="*",
        help="Dumps profile information for <stage>. If no stages are "
        + "provided, dumps the overall profiling information for this run.",
        dest="profiled_stages",
    )
    parser.add_argument(
        "-csv",
        "--csv_format",
        dest="csv",
        action="store_true",
        help="Whether data should be printed in CSV format. "
        + "This is currently only supported for profiling.",
    )
    parser.add_argument("--from", dest="source", help="Name of the start stage")
    parser.add_argument("--to", dest="dest", help="Name of the final stage")
    parser.add_argument(
        "--through",
        action="append",
        metavar="stage",
        default=[],
        help="Names of intermediate stages (repeatable option)",
    )
    parser.add_argument(
        "-o", dest="output_file", help="Name of the output file (default: STDOUT)"
    )
    # Provide configuration for stage options
    parser.add_argument(
        "-s",
        "--stage-val",
        help="Override stage configuration key-value pairs for this run",
        nargs=2,
        metavar=("key", "value"),
        dest="stage_dynamic_config",
        action="append",
    )

    # Alternatively, provide a TOML file with stage options
    parser.add_argument(
        "--stage-config",
        help="Path to a TOML file with stage configuration options",
        dest="config_file",
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

    return parser


def config_config(parser):
    parser.add_argument(
        "-e",
        "--edit",
        help="Edit the configuration file using $EDITOR",
        action="store_true",
    )
    parser.add_argument(
        "--remove",
        help=(
            "Delete the stored configuration."
            " Next invocation of `fud` will create a fresh config."
        ),
        action="store_true",
    )
    parser.add_argument("key", help="The key to perform an action on.", nargs="?")
    parser.add_argument("value", help="The value to write.", nargs="?")
    parser.add_argument(
        "-d", "--delete", help="Remove key from config.", action="store_true"
    )
    parser.add_argument(
        "-c", "--create", help="Create key in config.", action="store_true"
    )
    parser.set_defaults(command="config")


def config_info(parser):
    parser.set_defaults(command="info")


def config_check(parser):
    parser.set_defaults(command="check")
    # Take names of optional stages to check
    parser.add_argument(
        "stages", help="Names of stages to check", nargs="*", default=[]
    )


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
