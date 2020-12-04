from pathlib import Path
import argparse
import toml
import shutil
import logging as log
from termcolor import colored, cprint

from .stages import Source
from .config import Configuration
from .registry import Registry
from .stages import dahlia, futil, verilator, vcdump, systolic, mrxl, Source, SourceType
from . import exec, utils, config, errors


def register_stages(registry, config):
    """
    Register stages and command line flags required to generate the results.
    """
    # Dahlia
    registry.register(dahlia.DahliaStage(config))

    # MrXL
    registry.register(mrxl.MrXLStage(config))

    # Systolic Array
    registry.register(systolic.SystolicStage(config))

    # FuTIL
    registry.register(
        futil.FutilStage(config, 'verilog', '-b verilog',
                         'Compile FuTIL to Verilog instrumented for simulation'))
    registry.register(
        futil.FutilStage(config, 'futil-lowered', '-b futil',
                         'Compile FuTIL to FuTIL to remove all control and inline groups'))
    registry.register(
        futil.FutilStage(config, 'futil-noinline', '-b futil -d hole-inliner',
                         'Compile FuTIL to FuTIL to remove all control and inline groups'))

    registry.register(
        futil.FutilStage(config, 'futil-externalize', '-b futil -p externalize',
                         'Compile FuTIL to FuTIL to externalize all external memory primitives'))

    # Verilator
    registry.register(
        verilator.VerilatorStage(config, 'vcd',
                                 'Generate a VCD file from Verilog simulation'))
    registry.register(
        verilator.VerilatorStage(config, 'dat',
                                 'Generate a JSON file with final state of all memories'))

    # Vcdump
    registry.register(vcdump.VcdumpStage(config))


def config(args, config):
    if args.key is None:
        print(config.config_file)
        print()
        config.display()
    else:
        path = args.key.split(".")
        if args.value is None:
            # print out values
            res = config[path]
            if isinstance(res, dict):
                print(toml.dumps(res))
            else:
                print(res)
        else:
            config.touch(path)
            if not isinstance(config[path], list):
                config[path] = args.value
                config.commit()
            else:
                raise Exception("NYI: supporting updating lists")


def info(args, config):
    print(config.REGISTRY)


def check(args, config):
    config.launch_wizard()

    # check global
    futil_root = Path(config['global', 'futil_directory'])
    futil_str = colored(str(futil_root), 'yellow')
    cprint('global:', attrs=['bold'])
    if futil_root.exists():
        cprint(" ✔", 'green', end=' ')
        print(f"{futil_str} exists.")
    else:
        cprint(" ✖", 'red', end=' ')
        print(f"{futil_str} doesn't exist.")
    print()

    uninstalled = []
    # check executables in stages
    for name, stage in config['stages'].items():
        if 'exec' in stage:
            cprint(f'stages.{name}.exec:', attrs=['bold'])
            exec_path = shutil.which(stage['exec'])
            exec_name = colored(stage['exec'], 'yellow')
            if exec_path is not None or stage['exec'].startswith('cargo run'):
                cprint(" ✔", 'green', end=' ')
                print(f"{exec_name} installed.")
                if exec_path is not None and not Path(exec_path).is_absolute():
                    print(
                        f"   {exec_name} is a relative path and will not work from every directory.")
            else:
                uninstalled.append(name)
                cprint(" ✖", 'red', end=' ')
                print(f"{exec_name} not installed.")
            print()
    if len(uninstalled) > 0:
        bad_stages = colored(', '.join(uninstalled), 'red')
        verb = 'were' if len(uninstalled) > 1 else 'was'
        print(f"{bad_stages} {verb} not installed correctly.")
        print("Configuration instructions: https://capra.cs.cornell.edu/calyx/tools/fud.html#configuration")
        exit(-1)


def main():
    """Builds the command line argument parser,
    parses the arguments, and returns the results."""

    # Setup logging
    utils.logging_setup()

    parser = argparse.ArgumentParser(
        description="Driver to execute FuTIL and supporting toolchains"
    )
    subparsers = parser.add_subparsers()

    config_run(subparsers.add_parser(
        'exec',
        help="Execute one of the FuTIL-related tools",
        description="Execute one of the FuTIL-related tools",
        aliases=['e', 'ex']))
    config_config(subparsers.add_parser(
        'config',
        help="Configure environment variables for the driver",
        description="Configure environment variables for the driver",
        aliases=['c']))
    config_info(subparsers.add_parser(
        'info',
        help="Show information about execution stages",
        description="Show information about execution stages",
        aliases=['i']))
    config_check(subparsers.add_parser(
        'check',
        help="Check to make sure configuration is valid.",
        description="Check to make sure configuration is valid."))

    config = Configuration()

    # Build the registry.
    config.REGISTRY = Registry(config)
    register_stages(config.REGISTRY, config)

    args = parser.parse_args()

    if 'func' in args:
        try:
            args.func(args, config)
        except errors.FudError as e:
            log.error(e)
            exit(-1)
    else:
        parser.print_help()
        exit(-1)


def config_run(parser):
    parser.add_argument('--from', dest='source',
                        help='Name of the start stage')
    parser.add_argument('--to', dest='dest',
                        help='Name of the final stage')
    parser.add_argument('-o', dest='output_file',
                        help='Name of the outpfule file (default: STDOUT)')
    parser.add_argument(
        '-s',
        help='Override configuration key-value pairs for this run',
        nargs=2,
        metavar=('key', 'value'),
        dest='dynamic_config',
        action='append'
    )
    parser.add_argument('-n', '--dry-run',
                        action='store_true', dest='dry_run',
                        help='Show the execution stages and exit')
    parser.add_argument('-v', '--verbose', action='count', default=0,
                        help='Enable verbose logging')
    parser.add_argument('-q', '--quiet', action='store_true')
    parser.add_argument('input_file', help='Path to the input file', nargs='?')
    parser.set_defaults(func=exec.run_fud)


def config_config(parser):
    parser.add_argument(
        'key',
        help='The key to perform an action on.',
        nargs='?'
    )
    parser.add_argument(
        'value',
        help='The value to write.',
        nargs='?'
    )
    parser.set_defaults(func=config)


def config_info(parser):
    parser.set_defaults(func=info)


def config_check(parser):
    parser.set_defaults(func=check)
