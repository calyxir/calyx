from pathlib import Path
import appdirs
import toml
import sys
from pprint import PrettyPrinter

from .utils import eprint

wizard_data = {
    'global': {
        'futil_directory': 'Root Directory of FuTIL repository',
    }
}

DEFAULT_CONFIGURATION = {
    'global': {},
    'stages': {
        'futil': {
            'exec': 'futil',
            'file_extensions': ['.futil'],
            'flags': None
        },
        'dahlia': {
            'exec': 'dahlia',
            'file_extensions': ['.fuse', '.dahlia']
        },
        'verilog': {
            'exec': 'verilator',
            'file_extensions': ['.v', '.sv'],
            'cycle_limit': '5e8',
            'data': None
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


def wizard(table, data):
    for key in data.keys():
        if not isinstance(table, dict):
            table = {}

        if key not in table:
            while True:
                answer = input(f'{data[key]} is unset: ')
                path = Path(answer)
                if path.exists():
                    table[key] = str(path.resolve())
                    break
                else:
                    eprint(f"{path} doesn't exist.")

    return table


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
        self.launch_wizard()
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

    def launch_wizard(self):
        for key in self.config.keys():
            if key in wizard_data.keys():
                self.config[key] = wizard(self.config[key], wizard_data[key])

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
