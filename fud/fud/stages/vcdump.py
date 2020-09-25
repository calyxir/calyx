from fud.stages import Stage, Step, SourceType, Source

class VcdumpStage(Stage):
    def __init__(self, config):
        super().__init__('vcd', 'vcd_json', config)

    def define(self):
        main = Step(SourceType.File, SourceType.File)
        main.set_cmd(f'{self.cmd}')
        return [main]
