import importlib.util
import pathlib

from fud.stages import SourceType, Stage
from fud.utils import shell

from fud import errors


class NTTStage(Stage):
    """
    Stage to transform NTT configurations into Calyx programs.
    """

    name = "ntt"

    def __init__(self):
        super().__init__(
            src_state="ntt",
            target_state="calyx",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description="Compiles NTT configuration to Calyx.",
        )

    @staticmethod
    def pre_install():
        try:
            importlib.util.find_spec("prettytable")
        except ImportError:
            raise errors.FudRegisterError(
                "ntt",
                (
                    "`prettytable' library missing. "
                    "Install by running `pip install prettytable'."
                ),
            )

    @staticmethod
    def defaults():
        parent = pathlib.Path(__file__).parent.resolve()
        script_loc = parent / "../gen-ntt-pipeline.py"
        return {"exec": str(script_loc.resolve())}

    def _define_steps(self, input, builder, config):
        cmd = config["stages", self.name, "exec"]

        @builder.step(description=cmd)
        def run_ntt(conf: SourceType.Path) -> SourceType.Stream:
            return shell(f"{cmd} {conf!s}")

        return run_ntt(input)


# Export the defined stages to fud
__STAGES__ = [NTTStage]
