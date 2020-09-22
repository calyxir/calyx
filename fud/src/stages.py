import subprocess
from enum import Enum
from tempfile import NamedTemporaryFile, TemporaryDirectory
import json
from io import StringIO

from src.utils import debug
from src.json_to_dat import convert2dat, convert2json

class SourceType(Enum):
    Path = 1,
    Pipe = 2,
    Nothing = 3

# TODO: would be nice to not have to manually address each type
class Source:
    def __init__(self, data, source_type):
        self.data = data
        self.source_type = source_type

    def to_pipe(self):
        if self.source_type == SourceType.Path:
            self.data = open(self.data, 'r')
            self.source_type = SourceType.Pipe
        elif self.source_type == SourceType.Pipe:
            pass
        elif self.source_type == SourceType.Nothing:
            raise Exception('TODO: wronge source type error')

    def to_path(self):
        if self.source_type == SourceType.Path:
            pass
        elif self.source_type == SourceType.Pipe:
            with NamedTemporaryFile('wb', prefix='fud', delete=False) as tmpfile:
                for line in self.data:
                    tmpfile.write(line)
                self.data = tmpfile.name
                self.source_type = SourceType.Path
        elif self.source_type == SourceType.Nothing:
            raise Exception('TODO: wronge source type error')

class Stage:
    def __init__(self, name, target_stage, config):
        self.name = name
        self.target_stage = target_stage
        self.stage_config = config.find(['stages', self.name])
        self.cmd = self.stage_config['exec']

    def transform(self, input_source, output_source):
        return input_source

class DahliaStage(Stage):
    def __init__(self, config):
        super().__init__('dahlia', 'futil', config)

    def transform(self, input_source, output_source):
        debug(f"Running {self.name}")
        if input_source.source_type == SourceType.Path:
            proc = subprocess.Popen(
                f'{self.cmd} {input_source.data} -b futil --lower',
                shell=True,
                stdout=output_source.data,
                stderr=subprocess.PIPE
            )
            proc.wait()
            return (proc.stdout, proc.stderr, proc.returncode)
        else:
            raise Exception("TODO: error!")

class FutilStage(Stage):
    def __init__(self, config):
        super().__init__('futil', 'verilog', config)

    def transform(self, input_source, output_source):
        debug(f"Running {self.name}")

        if input_source.source_type == SourceType.Path:
            input_source.to_pipe()

        proc = subprocess.Popen(
            f'{self.cmd} -b verilog -l {self.stage_config["stdlib"]} --verilator',
            shell=True,
            stdin=input_source.data,
            stdout=output_source.data,
            stderr=subprocess.PIPE
        )
        proc.wait()
        return (proc.stdout, proc.stderr, proc.returncode)

class VerilatorStage(Stage):
    def __init__(self, config, mem):
        if mem == 'vcd' or mem == 'dat':
            self.vcd = mem == 'vcd'
            super().__init__('verilog', mem, config)
        else:
            raise Exception("mem has to be 'vcd' or 'dat'")

    def transform(self, input_source, output_source):
        debug(f"Running {self.name}")

        if input_source.source_type == SourceType.Pipe:
            input_source.to_path()

        exe_string = "--exe " + " --exe ".join(self.stage_config['testbench_files'])

        data = self.stage_config['data']

        with TemporaryDirectory() as tmpdir:
            data_prefix = ''
            # create data if necessary
            if data == None:
                with open(input_source.data, 'r') as verilog_src:
                    if 'readmemh' in verilog_src.read(): # the verilog expects data, but none has been provided
                        raise Exception("'verilog.data' needs to be set")
            else:
                with open(data) as f:
                    convert2dat(tmpdir, json.load(f), 'dat')
                    data_prefix = f'DATA={tmpdir}'

            verilator = " ".join([
                self.cmd,
                '-cc',
                '--trace',
                input_source.data,
                exe_string,
                '--top-module main',
                '--Mdir',
                tmpdir,
                '1>&2'
            ])
            make = f"make -j -C {tmpdir} -f Vmain.mk Vmain 1>&2"
            run = f"{data_prefix} {tmpdir}/Vmain {tmpdir}/output.vcd 1>&2"

            total = None
            if self.vcd:
                cat = f"cat {tmpdir}/output.vcd"
                total = ";\n".join([verilator, make, run, cat])
            else:
                total = ";\n".join([verilator, make, run])

            proc = subprocess.Popen(
                total,
                shell=True,
                stdout=output_source.data,
                stderr=subprocess.PIPE
            )
            proc.wait()

            out = proc.stdout
            if data != None and not self.vcd:
                mem = convert2json(tmpdir, "out")
                if output_source.source_type == SourceType.Path:
                    json.dump(mem, output_source.data, sort_keys=True, indent=2)
                elif output_source.source_type == SourceType.Pipe:
                    out = StringIO(json.dumps(mem, sort_keys=True, indent=2))
                elif output_source.source_type == SourceType.Nothing:
                    print(json.dumps(mem, sort_keys=True, indent=2))
                    out = None

            return (out, proc.stderr, proc.returncode)

class VcdumpStage(Stage):
    def __init__(self, config):
        super().__init__('vcd', 'vcd_json', config)

    def transform(self, inp, out):
        debug(f"Running {self.name}")
        if inp.source_type == SourceType.Path:
            proc = subprocess.Popen(
                f'{self.cmd} {inp.data}',
                shell=True,
                stdout=out.data,
                stderr=subprocess.PIPE
            )
            proc.wait()
            return (proc.stdout, proc.stderr)
        else:
            proc = subprocess.Popen(
                f'{self.cmd}',
                shell=True,
                stdin=inp.data,
                stdout=out.data,
                stderr=subprocess.PIPE
            )
            proc.wait()
        return (proc.stdout, proc.stderr, proc.returncode)
