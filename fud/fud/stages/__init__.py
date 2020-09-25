"""The definitions of fud stages."""

import subprocess
from enum import Enum
from tempfile import TemporaryFile, NamedTemporaryFile
from pathlib import Path
import sys
import logging as log

from ..utils import eprint


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

    def transform(self, input_src, dry_run=False):
        steps = self.define()
        ctx = {}

        prev_out = input_src
        err = None
        ret = None
        # loop until last step
        for step in steps:
            res = step.run(prev_out, ctx=ctx, dry_run=dry_run)
            (prev_out, err, ret) = res
            self.check_exit(ret, err)

        return (prev_out, err, ret)

    def check_exit(self, retcode, stderr):
        if retcode != 0:
            msg = f"Stage '{self.name}' had a non-zero exit code."
            n_dashes = (len(msg) - len(' stderr ')) // 2
            eprint(msg)
            eprint("-" * n_dashes, 'stderr', '-' * n_dashes)
            eprint(stderr, end='')
            exit(retcode)


class Step:
    def __init__(self, desired_input_type):
        self.func = None
        self.description = "No description provided."
        self.desired_input_type = desired_input_type

    def run(self, input_src, ctx={}, dry_run=False):
        if dry_run:
            print(f'     - {self.description}')
            return (None, None, 0)
        else:
            # convert input type to desired input type
            if self.desired_input_type == SourceType.Path:
                input_src.to_path()
            elif self.desired_input_type == SourceType.File:
                input_src.to_pipe()

            return self.func(input_src, ctx)

    def set_cmd(self, cmd):
        def f(inp, ctx):
            nonlocal cmd
            proc = None
            if inp.source_type == SourceType.Path:
                ctx['input_path'] = inp.data
                log.debug('  - [*] {}'.format(cmd.format(ctx=ctx)))
                proc = subprocess.Popen(
                    cmd.format(ctx=ctx),
                    shell=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                )
            else:
                log.debug('  - [*] pipe: {}'.format(cmd.format(ctx=ctx)))
                proc = subprocess.Popen(
                    cmd.format(ctx=ctx),
                    shell=True,
                    stdin=inp.data,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                )
            proc.wait()
            stderr = proc.stderr.read().decode('UTF-8')
            log.debug(stderr)
            return (
                Source(proc.stdout, SourceType.File),
                stderr,
                proc.returncode
            )
        self.func = f
        self.description = cmd

    def set_func(self, func, description):
        def f(inp, ctx):
            log.debug(description)
            # if out.source_type == SourceType.CreatePipe:
            #     out.data = TemporaryFile('r+')
            #     out.source_type = SourceType.File
            return func(inp, ctx)
        self.func = f
        self.description = description
