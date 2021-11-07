from fud.stages import Stage, SourceType
from fud.utils import shell
import pathlib


class NTTStage(Stage):
    """
    Stage to transform NTT configurations into Calyx programs.
    """

    name = "ntt"

    def __init__(self, config):
        super().__init__(
            src_state="ntt",
            target_state="futil",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            config=config,
            description="Compiles NTT configuration to Calyx.",
        )
        self.config["stages", self.name, "flags"] = None
        self.setup()

    @staticmethod
    def defaults():
        parent = pathlib.Path(__file__).parent.resolve()
        script_loc = parent / "../gen-ntt-pipeline.py"
        return {
            "exec": str(script_loc.resolve()),
            "flags": None,
        }

    def _define_steps(self, input_path):
        @self.step(description=self.cmd)
        def run_ntt(conf: SourceType.Path) -> SourceType.Stream:
            return shell(f"{self.cmd} {str(conf)}")

        return run_ntt(input_path)


# Export the defined stages to fud
__STAGES__ = [NTTStage]
