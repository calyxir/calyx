from pathlib import Path

from fud.stages import Stage, Step, SourceType


class MrXLStage(Stage):
    def __init__(self, config):
        super().__init__('mrxl', 'futil', config,
                         'Compiles MrXL to FuTIL.')

    def _define(self):
        main = Step(SourceType.Path)
        main.set_cmd(f'{self.cmd} {{ctx[input_path]}}')
        return [main]
