from fud.stages import Stage, Step, SourceType


class DahliaHLSStage(Stage):
    """
    Stage that transforms Dahlia programs to Vivado HLS C++.
    """

    def __init__(self, config):
        super().__init__(
            "dahlia", "vivado-hls", config, "Compiles Dahlia to Vivado HLS C++"
        )

    def _define(self):
        main = Step(SourceType.Path)
        main.set_cmd(
            " ".join([self.cmd, "--memory-interface ap_memory", "{ctx[input_path]}"])
        )
        return [main]
