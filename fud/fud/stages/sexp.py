from fud.stages import Stage, SourceType
from fud.utils import shell, unwrap_or


# A local constant used only within this file largely for organizational
# purposes and to avoid magic strings
_FILE_NAME = "data.json"


class SexpStage(Stage):

    name: str = "calyx-sexp"

    def __init__(
        self,
        desc="Parse Calyx programs with Coq semantics",
        output_type=SourceType.Stream,
        output_name="calyx-sexp",
    ):
        super().__init__(
            src_state="calyx",
            target_state="calyx-sexp",
            input_type=SourceType.Stream,
            output_type=output_type,
            description=desc,
        )

    def _define_steps(self, input_data, builder, config):
        script = config["stages", self.name, "exec"]
        # data_path = config["stages", "verilog", "data"]

# List of partial strings, with executable + flags in order and
        cmd = [
            script,
            unwrap_or(config["stages", self.name, "flags"], ""),
            # "--data" if data_path else "",
            # "{data_file}" if data_path else "",
            "{target}",
        ]

        cmd = " ".join(cmd)

        @builder.step(description=cmd)
        def to_sexp(
            target: SourceType.Path
        ) -> SourceType.Stream:
            """
            Serialize a Calyx program as s-exps
            """

            command = cmd.format(
                target=str(target)
            )

            return shell(command)

        # schedule
        result = to_sexp(input_data)
        # input_data is the terminal output of the sexp backend

        return result

    @staticmethod
    def pre_install():
        pass

    @staticmethod
    def defaults():
        return {}


__STAGES__ = [SexpStage]
