from pathlib import Path

from fud import config as cfg
from fud.stages import SourceType, Stage
from fud.utils import shell, unwrap_or


class SystolicStage(Stage):
    """
    Stage that invokes the Systolic Array frontend.
    """

    name = "systolic"

    def __init__(self):
        super().__init__(
            src_state="systolic",
            target_state="calyx",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description=(
                "Generates a matrix multiply using a systolic array architecture"
            ),
        )

    def _define_steps(self, input, builder, config):
        script = (
            Path(config["global", cfg.ROOT])
            / "frontends"
            / "systolic-lang"
            / "gen-systolic.py"
        )

        @builder.step(description=str(script))
        def run_systolic(input_path: SourceType.Path) -> SourceType.Stream:
            flags = unwrap_or(config["stages", self.name, "flags"], "")
            return shell(f"{str(script)} {str(input_path)} {flags}")

        return run_systolic(input)
