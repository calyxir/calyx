from pathlib import Path

from fud.stages import Stage, Step, SourceType
from .. import errors


class SystolicStage(Stage):
    def __init__(self, config):
        super().__init__('systolic', 'futil', config,
                         'Generates a matrix multiply using a systolic array architecture')

    def _define(self):
        main = Step(SourceType.Nothing)
        script = Path(self.config['global', 'futil_directory']) / 'frontends' / 'systolic-lang' / 'gen-systolic.py'
        if self.config['stages', self.name, 'flags'] is None:
            raise errors.MissingDynamicConfiguration('systolic.flags')
        flags = self.config['stages', self.name, 'flags']
        main.set_cmd(' '.join([str(script), flags]))
        return [main]
