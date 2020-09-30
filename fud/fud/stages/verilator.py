from tempfile import TemporaryDirectory
import json
from pathlib import Path
from io import BytesIO

from fud.stages import Stage, Step, SourceType, Source
from ..json_to_dat import convert2dat, convert2json
from .. import errors


class VerilatorStage(Stage):
    def __init__(self, config, mem, desc):
        if mem not in ['vcd', 'dat']:
            raise Exception("mem has to be 'vcd' or 'dat'")
        self.vcd = mem == 'vcd'
        super().__init__('verilog', mem, config, desc)

    def define(self):
        mktmp = Step(SourceType.Nothing)

        def f(inp, ctx):
            tmpdir = TemporaryDirectory()
            ctx['tmpdir'] = tmpdir.name
            ctx['tmpdir_obj'] = tmpdir
            return (inp, None, 0)
        mktmp.set_func(f, "Make temporary directory.")

        data = Step(SourceType.Path)
        data_path = self.stage_config['data']

        def f(inp, ctx):
            if data_path is None:
                with open(inp.data, 'r') as verilog_src:
                    # the verilog expects data, but none has been provided
                    if 'readmemh' in verilog_src.read():
                        raise errors.MissingDynamicConfiguration('verilog.data')
                    ctx['data_prefix'] = ''
            else:
                with open(data_path) as f:
                    convert2dat(ctx['tmpdir'], json.load(f), 'dat')
                    ctx['data_prefix'] = f'DATA={ctx["tmpdir"]}'
            return (inp, None, 0)
        data.set_func(f, "Convert json data to directory of .dat files.")

        verilator = Step(SourceType.Path)
        testbench_files = [
            str(Path(self.global_config['futil_directory']) / 'sim' / 'testbench.cpp'),
            str(Path(self.global_config['futil_directory']) / 'sim' / 'wrapper.cpp'),
        ]
        verilator.set_cmd(" ".join([
            self.cmd,
            '-cc',
            # Don't trace if we're only looking at memory outputs
            '--trace' if self.vcd else '',
            '{ctx[input_path]}',
            "--exe " + " --exe ".join(testbench_files),
            '--top-module main',  # TODO: make this use dynamic config
            '--Mdir',
            '{ctx[tmpdir]}',
            '1>&2'
        ]))

        make = Step(SourceType.Nothing)
        make.set_cmd("make -j -C {ctx[tmpdir]} -f Vmain.mk Vmain 1>&2")

        run = Step(SourceType.Nothing)
        run.set_cmd("{ctx[data_prefix]} {ctx[tmpdir]}/Vmain {ctx[tmpdir]}/output.vcd 1>&2")

        # switch later stages based on whether we are outputing vcd or mem files
        extract = Step(SourceType.Nothing)
        if self.vcd:
            def f(_inp, ctx):
                f = (Path(ctx['tmpdir']) / 'output.vcd').open('rb')
                return (Source(f, SourceType.File), None, 0)
            extract.set_func(f, "Read output.vcd.")
        else:
            def f(_inp, ctx):
                mem = convert2json(ctx['tmpdir'], 'out')
                buf = BytesIO(json.dumps(mem, indent=2).encode('UTF-8'))
                return (Source(buf, SourceType.File), None, 0)
            extract.set_func(f, "Convert output memories to json.")

        cleanup = Step(SourceType.File)

        def f(inp, ctx):
            ctx['tmpdir_obj'].cleanup()
            return (inp, None, 0)
        cleanup.set_func(f, "Cleanup tmp directory.")

        return [mktmp, data, verilator, make, run, extract, cleanup]
