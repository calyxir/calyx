from fud.stages import Stage, Step, SourceType


class VcdumpStage(Stage):
    def __init__(self, config):
        super().__init__('vcd', 'vcd_json', config)

    def define(self):
        main = Step(SourceType.File)
        main.set_cmd(f'{self.cmd} --pretty')
        return [main]
