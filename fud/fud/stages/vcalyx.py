from fud.stages import Stage, SourceType
from fud.utils import shell, unwrap_or


# A local constant used only within this file largely for organizational
# purposes and to avoid magic strings
_FILE_NAME = "data.json"


class VCalyxStage(Stage):

    name: str = "vcalyx"

    def __init__(
        self,
        desc="Parse Calyx programs with Coq semantics",
        output_type=SourceType.Stream,
        output_name="vcx",
    ):
        super().__init__(
            src_state="vcalyx",
            target_state=output_name,
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
        def vcx(
            target: SourceType.Path
        ) -> SourceType.Terminal:
            """
            Parse Calyx program in sexp form
            """

            command = cmd.format(
                target=str(target)
            )

            return shell(command)

        # schedule
        result = vcx(input_data)
        # input_data is the terminal output of the sexp backend

        return result

    @staticmethod
    def pre_install():
        pass

    @staticmethod
    def defaults():
        return {}


__STAGES__ = [VCalyxStage]
