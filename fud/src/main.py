from pathlib import Path
import subprocess
import argparse
from enum import Enum
import toml
import sys

from src.stages import *
from src.config import Configuration
from src.utils import debug

def discover_implied_stage(filename, config):
    suffix = Path(filename).suffix
    for (name, stage) in config['stages'].items():
        for ext in stage['file_extensions']:
            if suffix == ext:
                return name

    # no stages corresponding with this file extension where found
    return None

class Registry:
    def __init__(self, config):
        self.config = config

        # construct stage objects
        dahlia = DahliaStage(self.config)
        futil = FutilStage(self.config)
        verilator_vcd = VerilatorStage(self.config, 'vcd')
        verilator_dat = VerilatorStage(self.config, 'dat')
        vcdump = VcdumpStage(self.config)

        # make registry
        # TODO: assuming there is only a single path
        self.registry = {
            dahlia.name: (dahlia.target_stage, dahlia),
            futil.name: (futil.target_stage, futil),
            verilator_vcd.name: (verilator_vcd.target_stage, verilator_vcd),
            verilator_dat.name: (verilator_dat.target_stage, verilator_dat),
            vcdump.name: (vcdump.target_stage, vcdump),
        }

        debug(self.registry)

    def make_path(self, start, dest):
        path = []
        curr = start
        while curr != dest:
            (tar, stage) = self.registry[curr]
            path.append((tar, stage))
            curr = tar

        debug(path)
        return(path)

def run(args, config):

    # update the stages config with arguments provided via cmdline
    if args.dynamic_config != None:
        for key, value in args.dynamic_config:
            update(config.config['stages'], key.split('.'), value)

    registry = Registry(config)

    # find source
    source = args.source
    if source == None:
        source = discover_implied_stage(args.input_file, config.config)

    # find target
    target = args.dest
    if target == None:
        target = discover_implied_stage(args.output_file, config.config)

    debug(f"{source} -> {target}")
    path = registry.make_path(source, target)

    if len(path) == 0:
        # TODO: deal with case where input_file doesn't exist
        with open(args.input_file, 'r') as f:
            for line in f.readlines():
                print(line, end='')
    else:
        inp = Source(args.input_file, SourceType.Path)
        for (dest, stage) in path:
            debug(f"Going to {dest} with {stage.name}")
            out = None
            if dest == target:
                out = Source(None, SourceType.Nothing)
            else:
                out = Source(subprocess.PIPE, SourceType.Pipe)

            (result, stderr, retcode) = stage.transform(inp, out)

            if retcode != 0:
                debug(b''.join(stderr.readlines()).decode('ascii'))
                exit(retcode)

            inp = Source(result, SourceType.Pipe)

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
                debug(toml.dumps(res))
            else:
                debug(res)
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

    config_run(subparsers.add_parser('run', aliases=['r']))
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
    parser.add_argument('input_file')
    parser.set_defaults(func=run)

def config_config(parser):
    # TODO: add help for all of these options
    parser.add_argument('key', help='The key to perform an action on.', nargs='?')
    parser.add_argument('value', help='The value to write.', nargs='?')
    parser.set_defaults(func=config)
