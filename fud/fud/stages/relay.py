from pathlib import Path

from fud.stages import SourceType, Stage
from fud.utils import shell, unwrap_or
from fud import config as cfg


class RelayStage(Stage):
    """
    Stage that invokes the Relay frontend.
    """

    name = "relay"

    def __init__(self):
        super().__init__(
            src_state="relay",
            target_state="calyx",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description="Generates the Calyx program from the TVM Relay IR.",
        )

    def _define_steps(self, input, builder, config):
        script = (
            Path(config["global", cfg.ROOT])
            / "frontends"
            / "relay"
            / "relay_visitor.py"
        )

        @builder.step(description=str(script))
        def run_relay(input_path: SourceType.Path) -> SourceType.Stream:
            flags = unwrap_or(config["stages", self.name, "flags"], "")
            return shell(f"{str(script)} {str(input_path)} {flags}")

        return run_relay(input)
