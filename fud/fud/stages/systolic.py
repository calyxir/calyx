from pathlib import Path

from fud.stages import Source, SourceType, Stage, Step
from .. import errors


class SystolicStage(Stage):
    def __init__(self, config):
        super().__init__('systolic', 'futil', config,
                         'Generates a matrix multiply using a systolic array architecture')
        self.script = Path(self.config['global', 'futil_directory']) / 'frontends' / 'systolic-lang' / 'gen-systolic.py'

    def _define(self):
        main = Step(SourceType.Path)
        main.set_cmd(' '.join([
            str(self.script),
            '{ctx[input_path]}'
        ]))
        return [main]
