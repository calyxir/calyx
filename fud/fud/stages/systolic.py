from pathlib import Path

from fud.stages import Stage, Step, SourceType
from ..utils import unwrap_or


class SystolicStage(Stage):
    def __init__(self, config):
        super().__init__('systolic', 'futil', config,
                         'Generates a matrix multiply using a systolic array architecture')

    def _define(self):
        main = Step(SourceType.Nothing)
        script = Path(self.config['global', 'futil_directory']) / 'systolic-lang' / 'gen-systolic.py'
        flags = '-tl 2 -td 2 -ll 2 -ld 2'
        if self.config['stages', self.name, 'flags'] is not None:
            flags = self.config['stages', self.name, 'flags']
        main.set_cmd(' '.join([str(script), flags]))
        return [main]
