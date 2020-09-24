import subprocess
from enum import Enum
from tempfile import TemporaryFile, NamedTemporaryFile, TemporaryDirectory
import json
from io import StringIO, BufferedRWPair
from pathlib import Path
import sys
import os
import logging as log

from fud.json_to_dat import convert2dat, convert2json

class SourceType(Enum):
    Path = 1,
    File = 2,
    CreatePipe = 3,
    Nothing = 4

# TODO: would be nice to not have to manually address each type
class Source:
    def __init__(self, data, source_type):
        self.data = data
        self.source_type = source_type

    def pipe():
        # r, w = os.pipe()
        # data = BufferedRWPair(open(r, 'r'), open(w, 'w'))
        # f = TemporaryFile('r+')
        return Source(subprocess.PIPE, SourceType.CreatePipe)

    def to_pipe(self):
        if self.source_type == SourceType.Path:
            Path(self.data).touch()
            self.data = open(self.data, 'r+')
            self.source_type = SourceType.File
        elif self.source_type == SourceType.File:
            pass
        elif self.source_type == SourceType.Nothing:
            self.data = sys.stdout
            self.source_type = SourceType.File

    def to_path(self):
        if self.source_type == SourceType.Path:
            pass
        elif self.source_type == SourceType.File:
            with NamedTemporaryFile('wb', prefix='fud', delete=False) as tmpfile:
                for line in self.data:
                    tmpfile.write(line)
                self.data = tmpfile.name
                self.source_type = SourceType.Path
        elif self.source_type == SourceType.Nothing:
            pass

    def __repr__(self):
        return f"<Source {self.source_type} {self.data}>"

class Stage:
    def __init__(self, name, target_stage, config):
        self.name = name
        self.target_stage = target_stage
        self.stage_config = config.find(['stages', self.name])
        self.cmd = self.stage_config['exec']

    def define(self):
        """Not meant to be called by a user."""
        return None

    def transform(self, input_src, output_src, dry_run=False):
        steps = self.define()
        ctx = {}

        prev_out = input_src
        # loop until last step
        for step in steps[:-1]:
            res = step.run(prev_out, Source.pipe(), ctx=ctx, dry_run=dry_run)
            (prev_out, err, ret) = res

        res = steps[-1].run(prev_out, output_src, ctx=ctx, dry_run=dry_run)
        return res

class Step:
    def __init__(self, desired_input_type, desired_output_type):
        self.func = None
        self.description = "No description provided."
        self.desired_input_type = desired_input_type
        self.desired_output_type = desired_output_type

    def run(self, input_src, output_src, ctx={}, dry_run=False):
        if dry_run:
            log.info(f'     - {self.description}')
            return (None, None, 0)
        else:
            # convert input type to desired input type
            if self.desired_input_type == SourceType.Path:
                input_src.to_path()
            elif self.desired_input_type == SourceType.File:
                input_src.to_pipe()

            # convert output type to desired output type
            if self.desired_output_type == SourceType.Path:
                output_src.to_path()
            elif self.desired_output_type == SourceType.File:
                output_src.to_pipe()

            return self.func(input_src, output_src, ctx)

    def set_cmd(self, cmd):
        def f(inp, out, ctx):
            nonlocal cmd
            proc = None
            log.debug('cmd: {} {}'.format(inp, out))
            if inp.source_type == SourceType.Path:
                ctx['input_path'] = inp.data
                log.debug('  - [*] {}'.format(cmd.format(ctx=ctx)))
                proc = subprocess.Popen(
                    cmd.format(ctx=ctx),
                    shell=True,
                    stdout=out.data,
                    stderr=subprocess.PIPE,
                )
            else:
                log.debug('  - [*] {}'.format(cmd.format(ctx=ctx)))
                proc = subprocess.Popen(
                    cmd.format(ctx=ctx),
                    shell=True,
                    stdin=inp.data,
                    stdout=out.data,
                    stderr=subprocess.PIPE,
                )
            proc.wait()
            return (Source(proc.stdout, SourceType.File), proc.stderr, proc.returncode)
        self.func = f
        self.description = cmd

    def set_func(self, func, description):
        def f(inp, out, ctx):
            log.debug(description)
            if out.source_type == SourceType.CreatePipe:
                out.data = TemporaryFile('r+')
                out.source_type = SourceType.File
            return func(inp, out, ctx)
        self.func = f
        self.description = description

class DahliaStage(Stage):
    def __init__(self, config):
        super().__init__('dahlia', 'futil', config)

    def define(self):
        main = Step(SourceType.Path, SourceType.File)
        main.set_cmd(f'{self.cmd} {{ctx[input_path]}} -b futil --lower')
        return [main]

class FutilStage(Stage):
    def __init__(self, config):
        super().__init__('futil', 'verilog', config)

    def define(self):
        main = Step(SourceType.File, SourceType.File)
        main.set_cmd(f'{self.cmd} -b verilog -l {self.stage_config["stdlib"]} --verilator')
        return [main]

class VerilatorStage(Stage):
    def __init__(self, config, mem):
        if mem == 'vcd' or mem == 'dat':
            self.vcd = mem == 'vcd'
            super().__init__('verilog', mem, config)
        else:
            raise Exception("mem has to be 'vcd' or 'dat'")

    def define(self):
        mktmp = Step(SourceType.Nothing, SourceType.Nothing)
        def f(inp, out, ctx):
            tmpdir = TemporaryDirectory()
            ctx['tmpdir'] = tmpdir.name
            ctx['tmpdir_obj'] = tmpdir
            return (inp, None, 0)
        mktmp.set_func(f, "Make temporary directory.")

        data = Step(SourceType.Path, SourceType.Nothing)
        data_path = self.stage_config['data']
        def f(inp, out, ctx):
            if data_path == None:
                with open(inp.data, 'r') as verilog_src:
                    if 'readmemh' in verilog_src.read(): # the verilog expects data, but none has been provided
                        raise Exception("'verilog.data' needs to be set")
            else:
                with open(data_path) as f:
                    convert2dat(ctx['tmpdir'], json.load(f), 'dat')
                    ctx['data_prefix'] = f'DATA={ctx["tmpdir"]}'
            return (inp, None, 0)
        data.set_func(f, "Convert json data to directory of .dat files.")

        verilator = Step(SourceType.Path, SourceType.Nothing)
        verilator.set_cmd(" ".join([
            self.cmd,
            '-cc', '--trace',
            '{ctx[input_path]}',
            "--exe " + " --exe ".join(self.stage_config['testbench_files']),
            '--top-module main', # TODO: make this use dynamic config
            '--Mdir',
            '{ctx[tmpdir]}',
            '1>&2'
        ]))

        make = Step(SourceType.Nothing, SourceType.Nothing)
        make.set_cmd("make -j -C {ctx[tmpdir]} -f Vmain.mk Vmain 1>&2")

        run = Step(SourceType.Nothing, SourceType.Nothing)
        run.set_cmd("{ctx[data_prefix]} {ctx[tmpdir]}/Vmain {ctx[tmpdir]}/output.vcd 1>&2")

        # switch later stages based on whether we are outputing vcd or mem files
        extract = Step(SourceType.Nothing, SourceType.File)
        if self.vcd:
            def f(_inp, out, ctx):
                f = (Path(ctx['tmpdir']) / 'output.vcd').open('r')
                out.data.write(f.read())
                out.data.seek(0)
                return (out, None, 0)
            extract.set_func(f, "Read output.vcd.")
        else:
            def f(_inp, out, ctx):
                mem = convert2json(ctx['tmpdir'], 'out')
                out.data.write(json.dumps(mem, indent=2))
                out.data.seek(0)
                return (out, None, 0)
            extract.set_func(f, "Convert output memories to json.")

        cleanup = Step(SourceType.File, SourceType.File)
        def f(inp, out, ctx):
            ctx['tmpdir_obj'].cleanup()
            return (inp, None, 0)
        cleanup.set_func(f, "Cleanup tmp directory.")

        output = Step(SourceType.File, SourceType.File)
        def f(inp, out, ctx):
            out.data.write(inp.data.read())
            # TODO: figure out how to unify unseekable files (stdin/stdout) with seekable files
            # I probably want to do something different for connecting the end of a chain of steps with
            # an output source
            if out.data.seekable():
                out.data.seek(0)
            return(out, None, 0)
        output.set_func(f, "Output file.")
        return [mktmp, data, verilator, make, run, extract, cleanup, output]


class VcdumpStage(Stage):
    def __init__(self, config):
        super().__init__('vcd', 'vcd_json', config)

    def define(self):
        main = Step(SourceType.File, SourceType.File)
        main.set_cmd(f'{self.cmd}')
        return [main]
