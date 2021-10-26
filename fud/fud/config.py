import appdirs
import toml
import sys
import logging as log
from pathlib import Path
from pprint import PrettyPrinter

from .utils import eprint
from . import errors, external

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
            "debugger": {"flags": None},
        },
        "dahlia": {
            "exec": "dahlia",
            "file_extensions": [".fuse", ".dahlia"],
            "flags": None,
        },
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

    def get(self, keys):
        if isinstance(keys, str):
            keys = (keys,)

        data = self.data
        for k in keys:
            data = data.get(k)
            if data is None:
                return None

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

    Schema:
        The configuration file is serialized as a TOML file and contains the
        following data fields:

        1. global.futil_directory [required]. Location of the root folder of
           the Calyx repository.
        2. stages: A table containing information for each stage. For example,
           stage.verilog contains key-value pairs which encode the information
           for the verilog stage.
        3. externals: A table with (name, path) pairs for scripts that define
           external stages.
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
        Adds an external script that may define several stages.
        The path to the external script is stored in the [external] table of
        the configuration.
        """
        if not args.delete and args.path is not None:
            path = Path(args.path)
            if not path.exists() or not path.is_file():
                raise errors.FudRegisterError(args.name, f"`{path}' is not a file.")

            if self.config.get(["external", args.name]) is not None:
                raise errors.FudRegisterError(
                    args.name, f"External with name {args.name} already registered."
                )

            # Add the location of this script to externals in the
            # configuration.
            self[["externals", args.name]] = str(path.absolute())
            mod = external.validate_external_stage(args.name, self)

            print(f"Registering external script: {args.name}")
            for stage_class in mod.__STAGES__:
                print(f"  - Registering stage `{stage_class.name}'.")
                # Attach defaults for this stage if not present in the
                # configuration.
                for key, value in stage_class.defaults().items():
                    self["stages", args.name, key] = value

            self.commit()

        elif args.delete:
            if args.name in self[["externals"]]:
                print(f"Removing external script: {args.name}")
                # Only delete the stage if it's marked as an external
                del self[["externals", args.name]]
            else:
                log.error(f"No external script named `{args.name}'.")

    def __getitem__(self, keys):
        try:
            return self.config[keys]
        except KeyError:
            raise errors.UnsetConfiguration(keys)

    def get(self, keys):
        return self.config.get(keys)

    def __setitem__(self, keys, val):
        self.config[keys] = val

    def __delitem__(self, keys):
        del self.config[keys]

    def __contains__(self, keys):
        return keys in self.config

    def __str__(self):
        pp = PrettyPrinter(indent=2)
        return pp.pformat(self.config)
