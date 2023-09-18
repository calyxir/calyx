from typing import List, Set, Optional

import appdirs  # type: ignore
import toml
import sys
import logging as log
from pathlib import Path
from pprint import PrettyPrinter

from . import stages

from .utils import eprint
from . import errors, external, registry

# Key for the root folder
ROOT = "root"

# keys to prompt the user for
WIZARD_DATA = {
    "global": {
        ROOT: "Root Directory of Calyx repository",
    }
}

DEFAULT_CONFIGURATION = {
    "global": {},
    "externals": {},
    "stages": {
        "calyx": {
            "exec": "./target/debug/calyx",
            "file_extensions": [".futil"],
            "flags": None,
        },
        "interpreter": {
            "exec": "./target/debug/cider",
            "flags": "--raw ",
            "data": None,
            "round_float_to_fixed": True,
        },
        "debugger": {"flags": None},
        "dahlia": {
            "exec": "dahlia",
            "file_extensions": [".fuse", ".dahlia"],
            "flags": None,
        },
        "verilog": {
            "exec": "verilator",
            "file_extensions": [".v", ".sv"],
            "cycle_limit": int(5e8),
            "round_float_to_fixed": True,
            "data": None,
        },
        "vcd": {"exec": "vcdump", "file_extensions": [".vcd"]},
        "vcd_json": {"file_extensions": [".json"]},
        "jq": {"exec": "jq", "flags": None},
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
            "mode": "hw_emu",
            "device": "xilinx_u50_gen3x16_xdma_201920_3",
            "temp_location": "/tmp",
            "ssh_host": "",
            "ssh_username": "",
            "remote": None,
            "save_temps": None,
        },
        "fpga": {
            "data": None,
            "save_temps": None,
            "waveform": None,
        },
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

    def _merge_helper(self, store, cur_key=()):
        for k, v in store.items():
            if isinstance(v, dict):
                self._merge_helper(v, cur_key + (k,))
            else:
                self[cur_key + (k,)] = v

    def merge_dict(self, store):
        """Recursively Merge a dictionary into the current dictionary"""
        self._merge_helper(store)

    def __delitem__(self, keys):
        if isinstance(keys, str):
            keys = (keys,)

        data = self.data
        lastkey = keys[-1]
        for k in keys[:-1]:  # when assigning drill down to *second* last key
            if k in data:
                data = data[k]
        if lastkey in data:
            del data[lastkey]
        else:
            log.warn(f"`{'.'.join(keys)}' not found. Ignoring delete command.")

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

        1. global.root [required]. Location of the root folder of
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
        if not self.path.parent.exists():
            log.warn(f"{self.path.parent} doesn't exist. Creating it.")
        self.path.mkdir(parents=True, exist_ok=True)

        self.config_file = self.path / "config.toml"
        if not self.config_file.exists():
            self.config_file.touch()

        self.registry: registry.Registry = None

        # load the configuration file
        self.config = DynamicDict(toml.load(self.config_file))
        self.wizard_data = DynamicDict(WIZARD_DATA)
        self.fill_missing(DEFAULT_CONFIGURATION, self.config.data)
        if ("global", ROOT) not in self.config:
            log.warn(f"global.{ROOT} is not set in the configuration")

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
                # Ensure that the stage has a `pre_install` method.
                if "pre_install" not in dir(stage_class):
                    raise errors.FudRegisterError(
                        args.name,
                        (
                            f"Stage {stage_class.name} missing `pre_install()` method."
                            " If the stage has no pre-installation steps, add the "
                            " following stub to the class:\n"
                            "    @staticmethod\n"
                            "    def pre_install():\n"
                            "        pass\n"
                        ),
                    )
                stage_class.pre_install()
                # Ensure that the stage has a `defaults` method.
                if "defaults" not in dir(stage_class):
                    raise errors.FudRegisterError(
                        args.name,
                        (
                            f"Stage {stage_class.name} is missing `defaults()` method."
                            " If the stage has no default configuration, add the "
                            " following stub to the class:\n"
                            "    @staticmethod\n"
                            "    def defaults():\n"
                            "        return {}\n"
                        ),
                    )
                print(f"  - Registering stage `{stage_class.name}'.")
                # Attach defaults for this stage if not present in the
                # configuration.
                for key, value in stage_class.defaults().items():
                    self["stages", stage_class.name, key] = value

            self.commit()

        elif args.delete:
            if args.name in self[["externals"]]:
                print(f"Removing external script: {args.name}")
                # Only delete the stage if it's marked as an external
                del self[["externals", args.name]]
            else:
                log.warn(
                    f"Ignoring delete command, no external script named `{args.name}'."
                )

    def discover_implied_states(self, filename) -> str:
        """
        Use the mapping from filename extensions to stages to figure out which
        states were implied.
        Returns the input state on which the implied stage operates
        """
        suffix = Path(filename).suffix
        stages = []
        for name, stage in self["stages"].items():
            if "file_extensions" not in stage:
                continue
            if any([ext == suffix for ext in stage["file_extensions"]]):
                stages.append(name)

        # Implied stages only discovered when there is exactly one
        if len(stages) == 0:
            msg = f"`{suffix}' does not correspond to any known stage. "
            raise errors.UnknownExtension(msg, filename)
        elif len(stages) > 1:
            msg = f"`{suffix}' corresponds to multiple stages: {stages}. "
            raise errors.UnknownExtension(msg, filename)
        stage = stages[0]

        states = self.registry.get_states(stage)
        sources: Set[str] = set([source for (source, _) in states])

        # Only able to discover state if the stage has one input
        if len(sources) > 1:
            msg = f"Implied stage `{stage}' has multiple inputs: {states}. "
            raise errors.UnknownExtension(msg, filename)
        return sources.pop()

    def construct_path(
        self,
        source: Optional[str] = None,
        target: Optional[str] = None,
        input_file=None,
        output_file=None,
        through=[],
    ) -> List[stages.Stage]:
        """
        Construct the path of stages implied by the passed arguments.
        """
        # find source
        if source is None:
            source = self.discover_implied_states(input_file)
            log.debug(f"Inferred source state: {source}")

        # find target
        if target is None:
            target = self.discover_implied_states(output_file)
            log.debug(f"Inferred target state: {target}")

        path = self.registry.make_path(source, target, through)

        # If the path doesn't execute anything, it is probably an error.
        if len(path) == 0:
            raise errors.TrivialPath(source)

        return path

    def get(self, keys):
        return self.config.get(keys)

    def update_all(self, dict):
        self.config.merge_dict(dict)

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
