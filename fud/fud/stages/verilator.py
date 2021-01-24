from tempfile import TemporaryDirectory
import json
from pathlib import Path
from io import BytesIO

from fud.stages import Stage, Step, SourceType, Source
from ..json_to_dat import convert2dat, convert2json
from .. import errors


class VerilatorStage(Stage):
    def __init__(self, config, mem, desc):
        super().__init__('verilog', mem, config, desc)

        if mem not in ['vcd', 'dat']:
            raise Exception("mem has to be 'vcd' or 'dat'")
        self.vcd = mem == 'vcd'
        self.testbench_files = [
            str(Path(self.config['global', 'futil_directory']) / 'fud' / 'sim' / 'testbench.cpp'),
            str(Path(self.config['global', 'futil_directory']) / 'fud' / 'sim' / 'wrapper.cpp'),
        ]

    def mktmp_step(self):
        """
        Step 1: Make a temporary directory.
        """
        # Step 1: Make a new temporary directory.
        mktmp = Step(SourceType.Nothing)

        def f(inp, ctx):
            tmpdir = TemporaryDirectory()
            ctx['tmpdir'] = tmpdir.name
            ctx['tmpdir_obj'] = tmpdir
            return (inp, None, 0)
        mktmp.set_func(f, "Make temporary directory.")

        return mktmp

    def json_to_dat_step(self):
        """
        Step 2: Transform data from JSON to Dat.
        """
        data = Step(SourceType.Path)
        data_path = self.config['stages', self.name, 'data']

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

        return data

    def verilator_step(self):
        """
        Step 3: Build the design with verilator.
        """
        verilator = Step(SourceType.Path)
        verilator.set_cmd(" ".join([
            self.cmd,
            '-cc',
            '--trace',
            '{ctx[input_path]}',
            "--exe " + " --exe ".join(self.testbench_files),
            '--build',
            '--top-module', self.config['stages', self.name, 'top_module'],
            '--Mdir',
            '{ctx[tmpdir]}',
            '1>&2'
        ]))

        return verilator

    def verilator_run_step(self):
        """
        Step 3: Run the verilated design.
        """
        run = Step(SourceType.Nothing)
        run.set_cmd(" ".join([
            '{ctx[data_prefix]}',
            '{ctx[tmpdir]}/Vmain',
            '{ctx[tmpdir]}/output.vcd',
            str(self.config['stages', self.name, 'cycle_limit']),
            # Don't trace if we're only looking at memory outputs
            '--trace' if self.vcd else '',
            '1>&2'
        ]))

        return run

    def extract_data_step(self):
        """
        Step 4: Extract either VCD or DAT information from run.
        """
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
                buf = BytesIO(json.dumps(mem, indent=2, sort_keys=True).encode('UTF-8'))
                return (Source(buf, SourceType.File), None, 0)
            extract.set_func(f, "Convert output memories to json.")

        return extract

    def cleanup_step(self):
        """
        Step 5: Cleanup the temporary directory
        """
        cleanup = Step(SourceType.File)

        def f(inp, ctx):
            ctx['tmpdir_obj'].cleanup()
            return (inp, None, 0)
        cleanup.set_func(f, "Cleanup tmp directory.")

        return cleanup

    def _define(self):
        """
        Define all steps for running a verilator design.
        """
        mktmp = self.mktmp_step()
        data = self.json_to_dat_step()
        verilator = self.verilator_step()
        run = self.verilator_run_step()
        extract = self.extract_data_step()
        cleanup = self.cleanup_step()

        return [mktmp, data, verilator, run, extract, cleanup]
