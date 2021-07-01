from fud.stages import Stage, SourceType, Source
from pathlib import Path

from ..utils import shell, unwrap_or, TmpDir

_FILE_NAME = "data.json"


class InterpreterStage(Stage):
    def __init__(self, config, flags, desc):
        super().__init__(
            "interpreter",
            "interpreter-out",
            SourceType.Stream,
            SourceType.Stream,
            config,
            desc,
        )

        self.flags = flags
        self.data_path = self.config["stages", self.name, "data"]

        self.setup()

    def _define_steps(self, input_data):

        cmd = " ".join(
            [
                self.cmd,
                self.flags,
                "-l",
                self.config["global", "futil_directory"],
                "--data {data_file}" if self.data_path else "",
                "{target}",
            ]
        )

        @self.step()
        def mktmp() -> SourceType.Directory:
            """
            Make temporary directory to store Verilator build files.
            """
            return TmpDir()

        @self.step()
        def convert_json_to_interp_json(
            tmpdir: SourceType.Directory, json_path: SourceType.Stream
        ):
            """
            Creates a data file to initialze the interpreter memories
            """
            pass

        @self.step()
        def interpret(
            target: SourceType.Path, tmpdir: SourceType.Directory
        ) -> SourceType.Stream:
            """
            Invoke the interpreter
            """
            return shell(
                cmd.format(data_file=Path(tmpdir.name) / _FILE_NAME, target=str(target))
            )

        @self.step()
        def cleanup(tmpdir: SourceType.Directory):
            """
            Remove the temporary directory
            """
            tmpdir.remove()

        # schedule

        tmpdir = mktmp()

        if self.data_path is not None:
            convert_json_to_interp_json(
                tmpdir, Source(Path(self.data_path), SourceType.Path)
            )

        result = interpret(input_data, tmpdir)
        cleanup(tmpdir)

        return result
