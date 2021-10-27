import importlib.util
from pathlib import Path

from fud import errors


def validate_external_stage(stage, cfg):
    """
    Validate specification of an external stage in the configuration.
    """
    # get file location of stage and ensure that it exists
    location = cfg["externals", stage]
    if not Path(location).exists():
        raise errors.InvalidExternalStage(
            stage, f"No such file or directory: '{location}'"
        )

    # import the module from `location`
    spec = importlib.util.spec_from_file_location(stage, location)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)

    # check to make sure that module has `__STAGES__` defined.
    if not hasattr(mod, "__STAGES__"):
        raise errors.InvalidExternalStage(
            stage,
            "The module doesn't have attribute: '__STAGES__'."
            + "In order to export the defined stages, define an array of"
            + " stages called __STAGES__.",
        )

    return mod
