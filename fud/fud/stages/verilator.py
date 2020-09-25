from tempfile import TemporaryDirectory
import json
from pathlib import Path

from fud.stages import Stage, Step, SourceType
from ..json_to_dat import convert2dat, convert2json


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
            if data_path is None:
                with open(inp.data, 'r') as verilog_src:
                    # the verilog expects data, but none has been provided
                    if 'readmemh' in verilog_src.read():
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
            '--top-module main',  # TODO: make this use dynamic config
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
