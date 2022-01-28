from pathlib import Path

from fud.stages import SourceType, Stage
from fud.utils import shell


class RelayStage(Stage):
    """
    Stage that invokes the Relay frontend.
    """

    name = "relay"

    def __init__(self):
        super().__init__(
            src_state="relay",
            target_state="futil",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description="Generates the Calyx program from the TVM Relay IR.",
        )

    def _define_steps(self, input_path, config):
        script = (
            Path(config["global", "futil_directory"])
            / "frontends"
            / "relay"
            / "relay_visitor.py"
        )

        @self.step(description=str(script))
        def run_relay(input_path: SourceType.Path) -> SourceType.Stream:
            return shell(f"{str(script)} {str(input_path)}")

        return run_relay(input_path)
