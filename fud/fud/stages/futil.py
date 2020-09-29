from fud.stages import Stage, Step, SourceType


class FutilStage(Stage):
    def __init__(self, config, destination, flags, desc):
        self.flags = flags
        super().__init__('futil', destination, config, desc)

    def define(self):
        main = Step(SourceType.File)
        main.set_cmd(f'{self.cmd} -l {self.global_config["futil_directory"]} {self.flags} {{ctx[last]}}')
        main.last_context = {
            'last': '--force-color'
        }
        return [main]
