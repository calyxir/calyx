from pathlib import Path
import appdirs
import toml
import sys
from pprint import PrettyPrinter

DEFAULT_CONFIGURATION = {
    'stages': {
        'futil': {
            'exec': 'futil',
            'file_extensions': ['.futil'],
            'stdlib': '~/Research/futil'
        },
        'dahlia': {
            'exec': 'dahlia',
            'file_extensions': ['.fuse', '.dahlia']
        },
        'verilog': {
            'exec': 'verilator',
            'file_extensions': ['.v', '.sv'],
            'testbench_files': [
                '~/Research/futil/sim/testbench.cpp',
                '~/Research/futil/sim/wrapper.cpp'
            ],
            'cycle_limit': '5e8',
            'data': None  # look for data in current directory by default
        },
        'vcd': {
            'exec': 'vcdump',
            'file_extensions': ['.vcd']
        },
        'vcd_json': {
            'file_extensions': ['.json']
        },
        'dat': {
            'file_extensions': ['.dat']
        }
    }
}


class Configuration:
    def __init__(self):
        """Find the configuration file."""
        self.path = Path(appdirs.user_config_dir("fud"))
        self.path.mkdir(exist_ok=True)

        self.config_file = self.path / 'config.toml'
        self.config_file.touch()

        # load the configuration file
        self.config = toml.load(self.config_file)
        self.fill_missing(DEFAULT_CONFIGURATION, self.config)
        self.commit()

    def commit(self):
        toml.dump(self.config, self.config_file.open('w'))

    def display(self):
        toml.dump(self.config, sys.stdout)

    def fill_missing(self, default, config):
        if isinstance(default, dict):
            # go over all the keys in the default
            for key in default.keys():
                # if the key is not in the config, add it
                if key not in config:
                    config[key] = default[key]
                else:
                    config[key] = self.fill_missing(default[key], config[key])
        return config

    def find(self, path, pointer=None, total_path=None):
        # initiate pointer
        if pointer is None:
            pointer = self.config

        if total_path is None:
            total_path = path.copy()

        if len(path) == 0:
            return pointer
        else:
            key = path.pop(0)
            if key in pointer:
                return self.find(
                    path,
                    pointer=pointer[key],
                    total_path=total_path
                )
            else:
                p = '.'.join(total_path)
                raise Exception(f"'{p}' not found")

    def __str__(self):
        pp = PrettyPrinter(indent=2)
        return pp.pformat(self.config)
