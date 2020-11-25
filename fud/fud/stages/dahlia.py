from fud.stages import Stage, Step, SourceType
from ..utils import unwrap_or


class DahliaStage(Stage):
    """
    Stage that transforms Dahlia programs to FuTIL.
    """
    def __init__(self, config):
        super().__init__('dahlia', 'futil', config,
                         'Compiles a Dahlia program to FuTIL')

    def _define(self):
        main = Step(SourceType.Path)
        main.set_cmd(f'{self.cmd} ')
        main.set_cmd(' '.join([
            self.cmd,
            unwrap_or(self.config['stages', self.name, 'flags'], ''),
            '{{ctx[input_path]}} -b futil --lower'
        ]))
        return [main]
