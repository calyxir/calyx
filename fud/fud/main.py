import argparse
import toml
import logging as log

from .config import Configuration
from .registry import Registry
from .stages import dahlia, dahlia_hls, futil, verilator, vcdump, systolic, \
    mrxl, vivado, vivado_hls
from . import exec, utils, errors, check


def register_stages(registry, cfg):
    """
    Register stages and command line flags required to generate the results.
    """
    # Dahlia
    registry.register(dahlia.DahliaStage(cfg))
    registry.register(dahlia_hls.DahliaHLSStage(cfg))

    # MrXL
    registry.register(mrxl.MrXLStage(cfg))

    # Systolic Array
    registry.register(systolic.SystolicStage(cfg))

    # FuTIL
    registry.register(
        futil.FutilStage(
            cfg, 'verilog', '-b verilog',
            'Compile FuTIL to Verilog instrumented for simulation'
        ))
    registry.register(
        futil.FutilStage(
            cfg, 'synth-verilog', '-b verilog --synthesis -p external',
            'Compile FuTIL to synthesizable Verilog '
        ))
    registry.register(
        futil.FutilStage(
            cfg, 'futil-lowered', '-b futil',
            'Compile FuTIL to FuTIL to remove all control and inline groups'
        ))
    registry.register(
        futil.FutilStage(
            cfg, 'futil-noinline', '-b futil -d hole-inliner',
            'Compile FuTIL to FuTIL to remove all control and inline groups'
        ))
    registry.register(
        futil.FutilStage(cfg, 'futil-externalize', '-b futil -p externalize',
                         'Compile FuTIL to FuTIL to externalize all external memory primitives'
        ))

    # Verilator
    registry.register(
        verilator.VerilatorStage(
            cfg, 'vcd',
            'Generate a VCD file from Verilog simulation'
        ))
    registry.register(
        verilator.VerilatorStage(
            cfg, 'dat',
            'Generate a JSON file with final state of all memories'
        ))

    # Vivado / vivado hls
    registry.register(vivado.VivadoStage(cfg))
    registry.register(vivado.VivadoExtractStage(cfg))
    registry.register(vivado_hls.VivadoHLSStage(cfg))
    registry.register(vivado_hls.VivadoHLSExtractStage(cfg))

    # Vcdump
    registry.register(vcdump.VcdumpStage(cfg))


def display_config(args, cfg):
    if args.key is None:
        print(f"Configuration file location: {cfg.config_file}")
        print()
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
                raise Exception("NYI: supporting updating lists. " +
                                "Manually edit the file.")
            cfg.commit()


def info(args, cfg):
    print(cfg.REGISTRY)


def main():
    """Builds the command line argument parser,
    parses the arguments, and returns the results."""

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

    cfg = Configuration()

    # Build the registry.
    cfg.REGISTRY = Registry(cfg)
    register_stages(cfg.REGISTRY, cfg)

    args = parser.parse_args()

    # Setup logging
    utils.logging_setup(args)

    if 'func' in args:
        try:
            args.func(args, cfg)
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
    parser.add_argument(
        '-d',
        '--delete',
        help='Remove key from config.',
        action='store_true'
    )
    parser.set_defaults(func=display_config)


def config_info(parser):
    parser.set_defaults(func=info)


def config_check(parser):
    parser.set_defaults(func=check.check)
