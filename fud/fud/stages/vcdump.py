from fud.stages import Stage, Step, SourceType


class VcdumpStage(Stage):
    def __init__(self, config):
        super().__init__('vcd', 'vcd_json', config, 'Transform VCD file to JSON')

    def _define(self):
        main = Step(SourceType.File)
        main.set_cmd(f'{self.cmd} --pretty')
        return [main]
