from pathlib import Path

from fud.stages import SourceType, Stage


class SystolicStage(Stage):
    """
    Stage that invokes the Systolic Array frontend.
    """

    def __init__(self, config):
        super().__init__(
            "systolic",
            "futil",
            SourceType.Path,
            SourceType.Stream,
            config,
            "Generates a matrix multiply using a systolic array architecture",
        )
        self.script = (
            Path(self.config["global", "futil_directory"])
            / "frontends"
            / "systolic-lang"
            / "gen-systolic.py"
        )
        self.setup()

    def _define_steps(self, input_path):
        @self.step(
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description=str(self.script),
        )
        def run_systolic(step, input_path):
            return step.shell(f"{str(self.script)} {input_path}")

        return run_systolic(input_path)
