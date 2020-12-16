from tempfile import TemporaryDirectory
from pathlib import Path
from io import BytesIO

from fud.stages import Stage, Step, SourceType, Source
from ..vivado.extract import futil_extract


class VivadoStage(Stage):
    def __init__(self, config):
        super().__init__(
            'synth-verilog',
            'resource-estimate',
            config,
            'Runs synthesis on a Verilog program'
        )

    def _define(self):
        # make temporary directory
        mktmp = Step(SourceType.Nothing)

        def f(inp, ctx):
            tmpdir = TemporaryDirectory()
            ctx['tmpdir'] = tmpdir.name
            ctx['tmpdir_obj'] = tmpdir
            return (inp, None, 0)
        mktmp.set_func(f, "Make temporary directory.")

        # copy over files
        copy = Step(SourceType.Path)
        synth_files = [
            str(Path(self.config['global', 'futil_directory']) / 'fud' / 'synth' / 'synth.tcl'),
            str(Path(self.config['global', 'futil_directory']) / 'fud' / 'synth' / 'device.xdc'),
        ]
        copy.set_cmd(' '.join([
            'cp', ' '.join(synth_files), '{ctx[tmpdir]}', '&&',
            'cp {ctx[input_path]} {ctx[tmpdir]}/main.sv'
        ]))

        # run vivado
        vivado = Step(SourceType.Path)
        vivado.set_cmd(' '.join([
            'cd {ctx[tmpdir]}', '&&',
            ' vivado -mode batch -source synth.tcl >&2'
        ]))

        # extract
        extract = Step(SourceType.Nothing)

        def f(inp, ctx):
            res = futil_extract(Path(ctx['tmpdir']))
            return (Source(BytesIO(res.encode('UTF-8')), SourceType.File), None, 0)
        extract.set_func(f, 'Extract information.')

        return [mktmp, copy, vivado, extract]
