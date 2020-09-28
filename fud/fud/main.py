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
    registry.register(dahlia.DahliaStage(config))
    registry.register(
        futil.FutilStage(config, 'verilog', '-b verilog --verilator')
    )
    registry.register(futil.FutilStage(config, 'futil-lowered', '-b futil'))
    registry.register(
        futil.FutilStage(config, 'futil-noinline', '-b futil -p no-inline')
    )
    registry.register(verilator.VerilatorStage(config, 'vcd'))
    registry.register(verilator.VerilatorStage(config, 'dat'))
    registry.register(vcdump.VcdumpStage(config))


def run(args, config):
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

    # if we are doing a dry run, print out stages and exit
    if args.dry_run:
        print("Stages to run:")

    # check if input_file exists
    input_file = Path(args.input_file)
    if not input_file.exists():
        raise FileNotFoundError(input_file)

    if len(path) == 0:
        with input_file.open('r') as f:
            print(f.read())
    else:
        with Halo(
                spinner='dots',
                color='cyan',
                stream=sys.stderr,
                enabled=not utils.is_debug()) as sp:
            inp = Source(str(input_file), SourceType.Path)
            for ed in path:
                sp.start(f"{ed.stage.name} → {ed.stage.target_stage}")
                (result, stderr, retcode) = ed.stage.transform(
                    inp,
                    dry_run=args.dry_run
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

            if args.output_file is not None:
                with Path(args.output_file).open('w') as f:
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
        description="Fear, Uncertainty, Doubt. Beware of the FuTIL driver."
    )
    subparsers = parser.add_subparsers()

    config_run(subparsers.add_parser('exec', aliases=['e', 'ex']))
    config_config(subparsers.add_parser('config', aliases=['c']))
    config_info(subparsers.add_parser('info', aliases=['i']))

    config = Configuration()

    args = parser.parse_args()

    if 'func' in args:
        args.func(args, config)
    else:
        parser.print_help()
        exit(-1)


def config_run(parser):
    # TODO: add help for all of these options
    parser.add_argument('--from', dest='source')
    parser.add_argument('--to', dest='dest')
    parser.add_argument('-o', dest='output_file')
    parser.add_argument(
        '-s',
        nargs=2,
        metavar=('key', 'value'),
        dest='dynamic_config',
        action='append'
    )
    parser.add_argument('--dry-run', action='store_true', dest='dry_run')
    parser.add_argument('-v', '--verbose', action='count', default=0)
    parser.add_argument('input_file')
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
