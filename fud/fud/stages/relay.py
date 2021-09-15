from pathlib import Path

from fud.stages import SourceType, Stage
from fud.utils import shell


class RelayStage(Stage):
    """
    Stage that invokes the Relay frontend.
    """

    def __init__(self, config):
        super().__init__(
            "relay",
            "futil",
            SourceType.Path,
            SourceType.Stream,
            config,
            "Generates the Calyx program from the TVM Relay IR.",
        )
        self.script = (
            Path(self.config["global", "futil_directory"])
            / "frontends"
            / "relay"
            / "relay_visitor.py"
        )
        self.setup()

    def _define_steps(self, input_path):
        @self.step(description=str(self.script))
        def run_relay(input_path: SourceType.Path) -> SourceType.Stream:
            return shell(f"{str(self.script)} {str(input_path)}")

        return run_relay(input_path)
