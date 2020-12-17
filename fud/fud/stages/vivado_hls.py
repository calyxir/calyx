from tempfile import TemporaryDirectory
from pathlib import Path
from io import BytesIO

from fud.stages import Stage, Step, SourceType, Source
from ..vivado.extract import hls_extract


class VivadoHLSStage(Stage):
    def __init__(self, config):
        super().__init__(
            'cpp',
            'hls-estimate',
            config,
            'Runs HLS synthesis on a Dahlia program'
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
            str(Path(self.config['global', 'futil_directory']) / 'fud' / 'synth' / 'hls.tcl'),
            str(Path(self.config['global', 'futil_directory']) / 'fud' / 'synth' / 'fxp_sqrt.h'),
        ]
        copy.set_cmd(' '.join([
            'cp', ' '.join(synth_files), '{ctx[tmpdir]}', '&&',
            'cp {ctx[input_path]} {ctx[tmpdir]}/kernel.cpp'
        ]))

        # run vivado
        vivado_hls = Step(SourceType.Path)
        vivado_hls.set_cmd(' '.join([
            'cd {ctx[tmpdir]}', '&&',
            'vivado_hls -f hls.tcl >&2'
        ]))

        # extract
        extract = Step(SourceType.Nothing)

        def f(inp, ctx):
            res = hls_extract(Path(ctx['tmpdir']))
            return (Source(BytesIO(res.encode('UTF-8')), SourceType.File), None, 0)
        extract.set_func(f, 'Extract information.')

        return [mktmp, copy, vivado_hls, extract]
