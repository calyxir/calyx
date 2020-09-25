from fud.stages import Stage, Step, SourceType, Source

class FutilStage(Stage):
    def __init__(self, config, destination, flags):
        self.flags = flags
        super().__init__('futil', destination, config)

    def define(self):
        main = Step(SourceType.File)
        main.set_cmd(f'{self.cmd} -l {self.stage_config["stdlib"]} {self.flags}')
        return [main]
