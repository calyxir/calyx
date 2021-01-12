from pathlib import Path
import appdirs
import toml
import sys
from pprint import PrettyPrinter

from .utils import eprint
from . import errors

# Global registry. Initialized by main.py.
REGISTRY = None

wizard_data = {
    'global': {
        'futil_directory': 'Root Directory of FuTIL repository',
    }
}

DEFAULT_CONFIGURATION = {
    'global': {},
    'stages': {
        'futil': {
            'exec': './target/debug/futil',
            'file_extensions': ['.futil'],
            'flags': None
        },
        'dahlia': {
            'exec': 'dahlia',
            'file_extensions': ['.fuse', '.dahlia'],
            'flags': None
        },
        'mrxl': {
            'exec': 'mrxl',
            'file_extensions': ['.mrxl']
        },
        'verilog': {
            'exec': 'verilator',
            'file_extensions': ['.v', '.sv'],
            'cycle_limit': int(5e8),
            'top_module': "main",
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
        },
        'systolic': {
            'file_extensions': ['.systolic'],
            'flags': None
        },
        'vivado': {
            'exec': 'vivado'
        },
        'vivado_hls': {
            'exec': 'vivado_hls'
        }
    }
}


class DynamicDict:
    """Dynamically get/set nested dictionary keys of 'data' dict"""

    def __init__(self, data: dict):
        self.data = data

    def __getitem__(self, keys):
        if isinstance(keys, str):
            keys = (keys,)

        data = self.data
        for k in keys:
            data = data[k]
        return data

    def __setitem__(self, keys, val):
        if isinstance(keys, str):
            keys = (keys,)

        data = self.data
        lastkey = keys[-1]
        for k in keys[:-1]:  # when assigning drill down to *second* last key
            data = data[k]
        data[lastkey] = val

    def __contains__(self, keys):
        data = self.data
        for k in keys:
            if k in data:
                data = data[k]
            else:
                return False
        return True


def wizard(table, data):
    for key in data.keys():
        if not isinstance(table, dict):
            table = {}

        if key not in table:
            while True:
                answer = input(f'{data[key]} is unset (relative paths ok): ')
                path = Path(answer)
                if path.exists():
                    table[key] = str(path.resolve())
                    break
                else:
                    eprint(f"{path} doesn't exist.")

    return table


def rest_of_path(path):
    d = None
    for p in reversed(path):
        d = {
            p: d
        }
    return d


class Configuration:
    def __init__(self):
        """Find the configuration file."""
        self.path = Path(appdirs.user_config_dir("fud"))
        self.path.mkdir(exist_ok=True)

        self.config_file = self.path / 'config.toml'
        self.config_file.touch()

        # load the configuration file
        self.config = DynamicDict(toml.load(self.config_file))
        self.wizard_data = DynamicDict(wizard_data)
        self.fill_missing(DEFAULT_CONFIGURATION, self.config.data)
        # self.commit()

    def commit(self):
        toml.dump(self.config.data, self.config_file.open('w'))

    def display(self):
        toml.dump(self.config.data, sys.stdout)

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
        changed = False
        for key in self.config.data.keys():
            if key in self.wizard_data.data.keys():
                self.config.data[key] = wizard(
                    self.config[key],
                    wizard_data[key]
                )
                changed = True
        if changed:
            self.commit()

    def touch(self, path):
        if path in self.config:
            return
        for i in range(len(path), 0, -1):
            if path[:i] in self.config:
                self.config[path[:i]] = rest_of_path(path[i:])

    def __getitem__(self, keys):
        try:
            return self.config[keys]
        except KeyError:
            raise errors.UnsetConfiguration(keys)

    def __setitem__(self, keys, val):
        self.config[keys] = val

    def __contains__(self, keys):
        return keys in self.config

    def __str__(self):
        pp = PrettyPrinter(indent=2)
        return pp.pformat(self.config)
