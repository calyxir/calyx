from pathlib import Path
import argparse
import toml
import logging as log
from halo import Halo
import sys

from .stages import Source, SourceType
from .config import Configuration
from .registry import Registry
from . import errors
from .stages import dahlia, futil, verilator, vcdump
from . import utils


def discover_implied_stage(filename, config):
    """
    Use the mapping from filename extensions to stages to figure out which
    stage was implied.
    """
    if filename is None:
        raise Exception('TODO: No filename or type provided.')

    suffix = Path(filename).suffix
    for (name, stage) in config['stages'].items():
        for ext in stage['file_extensions']:
            if suffix == ext:
                return name

    # no stages corresponding with this file extension where found
    raise errors.UnknownExtension(filename)


def register_stages(registry, config):
    """
    Register stages and command line flags required to generate the results.
    """
    # Dahlia
    registry.register(dahlia.DahliaStage(config))

    # FuTIL
    registry.register(
        futil.FutilStage(config, 'verilog', '-b verilog --verilator',
                         'Compile FuTIL to Verilog instrumented for simulation'))
    registry.register(
        futil.FutilStage(config, 'futil-lowered', '-b futil',
                         'Compile FuTIL to FuTIL to remove all control and inline groups'))
    registry.register(
        futil.FutilStage(config, 'futil-noinline', '-b futil -p no-inline',
                         'Compile FuTIL to FuTIL to remove all control and inline groups'))

    # Verilator
    registry.register(
        verilator.VerilatorStage(config, 'vcd',
                                 'Generate a VCD file from Verilog simulation'))
    registry.register(
        verilator.VerilatorStage(config, 'dat',
                                 'Generate a JSON file with final state of all memories'))

    # Vcdump
    registry.register(vcdump.VcdumpStage(config))


def run(args, config):
    # check if input_file exists
    input_file = Path(args.input_file)
    if not input_file.exists():
        raise FileNotFoundError(input_file)

    # set verbosity level
    level = None
    if args.verbose <= 0:
        level = log.WARNING
    elif args.verbose <= 1:
        level = log.INFO
    elif args.verbose <= 2:
        level = log.DEBUG
    log.basicConfig(format="%(message)s", level=level)

    # update the stages config with arguments provided via cmdline
    if args.dynamic_config is not None:
        for key, value in args.dynamic_config:
            update(config.config['stages'], key.split('.'), value)

    registry = Registry(config)
    register_stages(registry, config)

    # find source
    source = args.source
    if source is None:
        source = discover_implied_stage(args.input_file, config.config)

    # find target
    target = args.dest
    if target is None:
        target = discover_implied_stage(args.output_file, config.config)

    path = registry.make_path(source, target)
    if path is None:
        raise errors.NoPathFound(source, target)

    # If the path doesn't execute anything, it is probably an error.
    if len(path) == 0:
        raise errors.TrivialPath(source)

    # if we are doing a dry run, print out stages and exit
    if args.dry_run:
        print("fud will perform the following steps:")

    # Pretty spinner.
    spinner_enabled = not (utils.is_debug() or args.dry_run or args.quiet)
    # Execute the path transformation specification.
    with Halo(
            spinner='dots',
            color='cyan',
            stream=sys.stderr,
            enabled=spinner_enabled) as sp:
        inp = Source(str(input_file), SourceType.Path)
        for i, ed in enumerate(path):
            sp.start(f"{ed.stage.name} → {ed.stage.target_stage}")
            (result, stderr, retcode) = ed.stage.transform(
                inp,
                dry_run=args.dry_run,
                last=i == (len(path) - 1)
            )
            inp = result

            if retcode == 0:
                if log.getLogger().level <= log.INFO:
                    sp.succeed()
            else:
                if log.getLogger().level <= log.INFO:
                    sp.fail()
                else:
                    sp.stop()
                utils.eprint(stderr)
                exit(retcode)
        sp.stop()

        # return early when there's a dry run
        if args.dry_run:
            return

        if args.output_file is not None:
            with Path(args.output_file).open('wb') as f:
                f.write(inp.data.read())
        else:
            print(inp.data.read().decode('UTF-8'))


def update(d, path, val):
    if len(path) == 0:
        d = val
    else:
        key = path.pop(0)  # get first element in path
        d[key] = update(d[key], path, val)
    return d


def config(args, config):
    if args.key is None:
        print(config.config_file)
        print()
        config.display()
    else:
        path = args.key.split(".")
        if args.value is None:
            # print out values
            res = config.find(path)
            if isinstance(res, dict):
                print(toml.dumps(res))
            else:
                print(res)
        else:
            if not isinstance(config.find(path.copy()), list):
                update(config.config, path, args.value)
                config.commit()
            else:
                raise Exception("NYI: supporting updating lists")


def info(args, config):
    registry = Registry(config)
    register_stages(registry, config)
    print(registry)


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

    config = Configuration()

    args = parser.parse_args()

    if 'func' in args:
        try:
            args.func(args, config)
        except errors.FudError as e:
            print('Error: ' + str(e))
    else:
        parser.print_help()
        exit(-1)


def config_run(parser):
    # TODO: add help for all of these options
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
    parser.add_argument('input_file', help='Path to the input file')
    parser.set_defaults(func=run)


def config_config(parser):
    # TODO: add help for all of these options
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
    # TODO: add help for all these options
    parser.set_defaults(func=info)
