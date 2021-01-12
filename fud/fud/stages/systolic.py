from pathlib import Path

from fud.stages import Source, SourceType, Stage, Step
from .. import errors


class SystolicStage(Stage):
    def __init__(self, config):
        super().__init__('systolic', 'futil', config,
                         'Generates a matrix multiply using a systolic array architecture')

    def _define(self):
        main = Step(SourceType.Nothing)
        script = Path(self.config['global', 'futil_directory']) / 'frontends' / 'systolic-lang' / 'gen-systolic.py'
        flags = ""
        def f(inp, _ctx):
            if inp.data is not None:
                return ' '.join([
                    str(script),
                    inp.data
                ])
            elif self.config['stages', self.name, 'flags'] is None:
                raise errors.MissingDynamicConfiguration('systolic.flags')
            else:
                return ' '.join([
                    str(script),
                    self.config['stages', self.name, 'flags']
                ])
        main.set_dynamic_cmd(f, f"{str(script)} {{ctx[input_path]}}")
        return [main]
