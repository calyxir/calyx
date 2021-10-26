from pathlib import Path

from fud.stages import SourceType, Stage
from fud.utils import shell


class SystolicStage(Stage):
    """
    Stage that invokes the Systolic Array frontend.
    """

    name = "systolic"

    def __init__(self, config):
        super().__init__(
            src_state="systolic",
            target_state="futil",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            config=config,
            description="Generates a matrix multiply using a systolic array architecture",
        )
        self.script = (
            Path(self.config["global", "futil_directory"])
            / "frontends"
            / "systolic-lang"
            / "gen-systolic.py"
        )
        self.setup()

    def _define_steps(self, input_path):
        @self.step(description=str(self.script))
        def run_systolic(input_path: SourceType.Path) -> SourceType.Stream:
            return shell(f"{str(self.script)} {str(input_path)}")

        return run_systolic(input_path)
