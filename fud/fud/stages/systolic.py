from pathlib import Path

from fud.stages import SourceType, Stage
from fud.utils import shell


class SystolicStage(Stage):
    """
    Stage that invokes the Systolic Array frontend.
    """

    name = "systolic"

    def __init__(self):
        super().__init__(
            src_state="systolic",
            target_state="futil",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description=(
                "Generates a matrix multiply using a systolic array architecture"
            ),
        )

    def _define_steps(self, input, builder, config):
        script = str(
            Path(config["global", "futil_directory"])
            / "frontends"
            / "systolic-lang"
            / "gen-systolic.py"
        )
        cmd = " ".join([script, "{input_path}"])

        @builder.step(description=cmd)
        def run_systolic(input_path: SourceType.Path) -> SourceType.Stream:
            return shell(cmd.format(input_path=str(input_path)))

        return run_systolic(input)
