import appdirs
import toml
import sys
import logging as log
from pathlib import Path
from pprint import PrettyPrinter

from .utils import eprint
from . import errors

# Global registry. Initialized by main.py.
REGISTRY = None

# keys to prompt the user for
WIZARD_DATA = {
    "global": {
        "futil_directory": "Root Directory of Calyx repository",
    }
}

DEFAULT_CONFIGURATION = {
    "global": {},
    "stages": {
        "futil": {
            "exec": "./target/debug/futil",
            "file_extensions": [".futil"],
            "flags": None,
        },
        "interpreter": {
            "exec": "./target/debug/interp",
            "flags": None,
            "data": None,
            "round_float_to_fixed": True,
        },
        "dahlia": {
            "exec": "dahlia",
            "file_extensions": [".fuse", ".dahlia"],
            "flags": None,
        },
        "mrxl": {"exec": "mrxl", "file_extensions": [".mrxl"]},
        "verilog": {
            "exec": "verilator",
            "file_extensions": [".v", ".sv"],
            "cycle_limit": int(5e8),
            "top_module": "main",
            "round_float_to_fixed": True,
            "data": None,
        },
        "vcd": {"exec": "vcdump", "file_extensions": [".vcd"]},
        "vcd_json": {"file_extensions": [".json"]},
        "dat": {"file_extensions": [".dat"]},
        "relay": {"file_extensions": [".relay"], "flags": None},
        "systolic": {"file_extensions": [".systolic"], "flags": None},
        "synth-verilog": {
            "exec": "vivado",
            "ssh_host": "",
            "ssh_username": "",
            "remote": None,
        },
        "vivado-hls": {
            "exec": "vivado_hls",
            "ssh_host": "",
            "ssh_username": "",
            "remote": None,
        },
        "xclbin": {
            "file_extensions": [".xclbin"],
            "mode": "hw_emu",
            "device": "xilinx_u50_gen3x16_xdma_201920_3",
            "temp_location": "/tmp",
            "ssh_host": "",
            "ssh_username": "",
            "save_temps": None,
        },
        "wdb": {
            "file_extensions": [".wdb"],
            "mode": "hw_emu",
            "ssh_host": "",
            "ssh_username": "",
            "host": None,
            "save_temps": None,
            "xilinx_location": "/scratch/opt/Xilinx/Vitis/2020.2",
            "xrt_location": "/opt/xilinx/xrt",
        },
        "fpga": {"data": None},
    },
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
            # if key exists, drill down
            if k in data:
                data = data[k]
            # else make a new empty dictionary, then drill down
            else:
                data[k] = {}
                data = data[k]
        data[lastkey] = val

    def __delitem__(self, keys):
        if isinstance(keys, str):
            keys = (keys,)

        data = self.data
        lastkey = keys[-1]
        for k in keys[:-1]:  # when assigning drill down to *second* last key
            data = data[k]
        del data[lastkey]

    def __contains__(self, keys):
        data = self.data
        for k in keys:
            if k in data:
                data = data[k]
            else:
                return False
        return True


def wizard(table, data):
    """
    Prompt the user for unset keys as specified in `data`.
    """
    for key in data.keys():
        if not isinstance(table, dict):
            table = {}

        if key not in table:
            while True:
                answer = input(f"{data[key]} is unset (relative paths ok): ")
                path = Path(answer).expanduser()
                if path.exists():
                    table[key] = str(path.resolve())
                    break

                eprint(f"{path} doesn't exist.")

    return table


class Configuration:
    """
    Wraps the configuration file and provides methods for committing
    data, displaying configuration data, accessing data, and prompting
    the user for unset keys.
    """

    def __init__(self):
        """Find the configuration file."""
        self.path = Path(appdirs.user_config_dir("fud"))
        self.path.mkdir(exist_ok=True)

        self.config_file = self.path / "config.toml"
        self.config_file.touch()

        # load the configuration file
        self.config = DynamicDict(toml.load(self.config_file))
        self.wizard_data = DynamicDict(WIZARD_DATA)
        self.fill_missing(DEFAULT_CONFIGURATION, self.config.data)
        if ("global", "futil_directory") not in self.config:
            log.warn("global.futil_directory is not set in the configuration")

    def commit(self):
        """
        Commit the current configuration to a file.
        """
        toml.dump(self.config.data, self.config_file.open("w"))

    def display(self):
        """
        Display the current configuration.
        """
        toml.dump(self.config.data, sys.stdout)

    def fill_missing(self, default, config):
        """
        Add keys that are defined in the default config but not in
        the user provided config.
        """
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
        """
        Launch the wizard to prompt user for unset keys.
        """
        changed = False
        for key in self.config.data.keys():
            if key in self.wizard_data.data.keys():
                self.config.data[key] = wizard(self.config[key], WIZARD_DATA[key])
                changed = True
        if changed:
            self.commit()

    def setup_external_stage(self, args):
        """
        Adds an `external-stages` entry for a stage passed
        via the command line.
        """
        if not args.delete and args.path is not None:
            path = Path(args.path)
            if not path.exists():
                raise FileNotFoundError(path)
            stage = {"location": str(path.absolute())}
            self["external-stages", args.name] = stage
            self.commit()
        elif args.delete:
            if args.name in self[["external-stages"]]:
                del self["external-stages", args.name]
                self.commit()

    def __getitem__(self, keys):
        try:
            return self.config[keys]
        except KeyError:
            raise errors.UnsetConfiguration(keys)

    def __setitem__(self, keys, val):
        self.config[keys] = val

    def __delitem__(self, keys):
        del self.config[keys]

    def __contains__(self, keys):
        return keys in self.config

    def __str__(self):
        pp = PrettyPrinter(indent=2)
        return pp.pformat(self.config)
