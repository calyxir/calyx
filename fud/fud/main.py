from pathlib import Path
import subprocess
import argparse
from enum import Enum
import toml
import sys
import logging as log

from fud.stages import *
from fud.config import Configuration
from fud.registry import Registry

def discover_implied_stage(filename, config):
    if filename == None:
        raise Exception('TODO: No filename or type provided.')

    suffix = Path(filename).suffix
    for (name, stage) in config['stages'].items():
        for ext in stage['file_extensions']:
            if suffix == ext:
                return name

    # no stages corresponding with this file extension where found
    raise Exception(f"TODO: real message: {filename} doesn't correspond to a known extension.")

def register_stages(registry, config):
    dahlia = DahliaStage(config)
    futil = FutilStage(config)
    verilator_vcd = VerilatorStage(config, 'vcd')
    verilator_dat = VerilatorStage(config, 'dat')
    vcdump = VcdumpStage(config)

    registry.register(dahlia.name, dahlia.target_stage, dahlia)
    registry.register(futil.name, futil.target_stage, futil)
    registry.register(verilator_vcd.name, verilator_vcd.target_stage, verilator_vcd)
    registry.register(verilator_dat.name, verilator_dat.target_stage, verilator_dat)
    registry.register(vcdump.name, vcdump.target_stage, vcdump)

def run(args, config):
    # set verbosity level
    l = log.getLogger()
    if args.verbose <= 0:
        l.setLevel(log.WARNING)
    elif args.verbose <= 1:
        l.setLevel(log.INFO)
    elif args.verbose <= 2:
        l.setLevel(log.DEBUG)

    # update the stages config with arguments provided via cmdline
    if args.dynamic_config != None:
        for key, value in args.dynamic_config:
            update(config.config['stages'], key.split('.'), value)

    registry = Registry(config)
    register_stages(registry, config)

    # find source
    source = args.source
    if source == None:
        source = discover_implied_stage(args.input_file, config.config)

    # find target
    target = args.dest
    if target == None:
        target = discover_implied_stage(args.output_file, config.config)

    path = registry.make_path(source, target)

    # if we are doing a dry run, print out stages and exit
    if args.dry_run:
        print("Stages to run:")

    if len(path) == 0:
        # TODO: deal with case where input_file doesn't exist
        with open(args.input_file, 'r') as f:
            print(f.read())
    else:
        inp = Source(args.input_file, SourceType.Path)
        for ed in path:
            out = None
            if ed.dest == target:
                if args.output_file != None:
                    out = Source(args.output_file, SourceType.Path)
                else:
                    out = Source(sys.stdout, SourceType.File)
            else:
                out = Source.pipe()

            print(f" [+] {ed.stage.name} -> {ed.stage.target_stage}")
            (result, stderr, retcode) = ed.stage.transform(inp, out, dry_run=args.dry_run)

            if retcode != 0:
                print(b''.join(stderr.readlines()).decode('ascii'))
                exit(retcode)

            inp = result

# TODO: is there a nice way to merge update and find?
def update(d, path, val):
    if len(path) == 0:
        d = val
    else:
        key = path.pop(0) # get first element in path
        d[key] = update(d[key], path, val)
    return d

def find(d, path):
    if len(path) == 0:
        return d
    else:
        key = path.pop(0) # get first element in path
        return find(d[key], path)

def config(args, config):
    if args.key == None:
        config.display()
    else:
        path = args.key.split(".")
        if args.value == None:
            # print out values
            res = find(config.config, path)
            if isinstance(res, dict):
                print(toml.dumps(res))
            else:
                print(res)
        else:
            if path[-1] == 'exec':
                update(config.config, args.key.split("."), args.value)
                config.commit()
            else:
                raise Exception("NYI: Can't update anything besides exec yet")

def main():
    """Builds the command line argument parser, parses the arguments, and returns the results."""
    parser = argparse.ArgumentParser(description="Fear, Uncertainty, Doubt. Beware of the FuTIL driver.")
    subparsers = parser.add_subparsers()

    config_run(subparsers.add_parser('exec', aliases=['e', 'ex']))
    config_config(subparsers.add_parser('config', aliases=['c']))

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
    parser.add_argument('-s', nargs=2, metavar=('key', 'value'), dest='dynamic_config', action='append')
    parser.add_argument('--dry-run', action='store_true', dest='dry_run')
    parser.add_argument('-v', '--verbose', action='count', default=0)
    parser.add_argument('input_file')
    parser.set_defaults(func=run)

def config_config(parser):
    # TODO: add help for all of these options
    parser.add_argument('key', help='The key to perform an action on.', nargs='?')
    parser.add_argument('value', help='The value to write.', nargs='?')
    parser.set_defaults(func=config)
