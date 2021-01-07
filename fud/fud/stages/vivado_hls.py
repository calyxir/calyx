from io import BytesIO
from pathlib import Path
from tempfile import TemporaryDirectory

from fud.stages import Source, SourceType, Stage, Step

from ..vivado.extract import hls_extract


class VivadoHLSStage(Stage):
    def __init__(self, config):
        super().__init__(
            "cpp", "hls-files", config, "Runs HLS synthesis on a Dahlia program"
        )

    def _define(self):
        # make temporary directory
        mktmp = Step(SourceType.Nothing)

        def f(inp, ctx):
            tmpdir = TemporaryDirectory()
            ctx["tmpdir"] = tmpdir.name
            ctx["tmpdir_obj"] = tmpdir
            return (inp, None, 0)

        mktmp.set_func(f, "Make temporary directory.")

        # copy over files
        move = Step(SourceType.Path)
        synth_files = [
            str(
                Path(self.config["global", "futil_directory"])
                / "fud"
                / "synth"
                / "hls.tcl"
            ),
            str(
                Path(self.config["global", "futil_directory"])
                / "fud"
                / "synth"
                / "fxp_sqrt.h"
            ),
        ]
        move.set_cmd(
            " ".join(
                [
                    "cp",
                    " ".join(synth_files),
                    "{ctx[tmpdir]}",
                    "&&",
                    "cp {ctx[input_path]} {ctx[tmpdir]}/kernel.cpp",
                ]
            )
        )

        # run vivado
        vivado_hls = Step(SourceType.Path)
        vivado_hls.set_cmd(
            " ".join(["cd {ctx[tmpdir]}", "&&", "vivado_hls -f hls.tcl >&2"])
        )

        # output directory
        output = Step(SourceType.Nothing)

        def f(inp, ctx):
            return (Source(ctx["tmpdir_obj"], SourceType.TmpDir), None, 0)

        output.set_func(f, "Output synthesis directory.")

        return [mktmp, move, vivado_hls, output]


class VivadoHLSExtractStage(Stage):
    def __init__(self, config):
        super().__init__(
            "hls-files",
            "hls-estimate",
            config,
            "Runs HLS synthesis on a Dahlia program",
        )

    def _define(self):
        # make temporary directory
        extract = Step(SourceType.Nothing)

        def f(inp, ctx):
            res = None
            if inp.source_type == SourceType.TmpDir:
                res = hls_extract(Path(inp.data.name))
            else:
                res = hls_extract(Path(inp.data))
            return (Source(BytesIO(res.encode("UTF-8")), SourceType.File), None, 0)

        extract.set_func(f, "Extract information.")

        return [extract]
