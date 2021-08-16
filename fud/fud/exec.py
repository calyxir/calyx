import logging as log
import shutil
import sys
from pathlib import Path

from halo import Halo

from . import errors, utils
from .stages import Source, SourceType


def discover_implied_stage(filename, config, possible_dests=None):
    """
    Use the mapping from filename extensions to stages to figure out which
    stage was implied.
    """
    if filename is None:
        raise errors.NoInputFile(possible_dests)

    suffix = Path(filename).suffix
    for (name, stage) in config["stages"].items():
        if "file_extensions" in stage:
            for ext in stage["file_extensions"]:
                if suffix == ext:
                    return name

    # no stages corresponding with this file extension where found
    raise errors.UnknownExtension(filename)


def construct_path(args, config):
    """
    Construct the path of stages implied by the passed arguments.
    """
    # find source
    source = args.source
    if source is None:
        source = discover_implied_stage(args.input_file, config)

    # find target
    target = args.dest
    if target is None:
        target = discover_implied_stage(
            args.output_file, config, possible_dests=config.REGISTRY.nodes[source]
        )

    path = config.REGISTRY.make_path(source, target)
    if path is None:
        raise errors.NoPathFound(source, target)

    # If the path doesn't execute anything, it is probably an error.
    if len(path) == 0:
        raise errors.TrivialPath(source)

    return path


def run_fud(args, config):
    """
    Execute all the stages implied by the passed `args`
    """
    # check if input_file exists
    input_file = None
    if args.input_file is not None:
        input_file = Path(args.input_file)
        if not input_file.exists():
            raise FileNotFoundError(input_file)

    path = construct_path(args, config)

    # check if we need `-o` specified
    if path[-1].stage.output_type == SourceType.Directory and args.output_file is None:
        raise errors.NeedOutputSpecified(path[-1].stage)

    # if we are doing a dry run, print out stages and exit
    if args.dry_run:
        print("fud will perform the following steps:")
        for ed in path:
            print(f"Stage: {ed.stage.name}")
            ed.stage.dry_run()
        return

    # spinner is disabled if we are in debug mode, doing a dry_run, or are in quiet mode
    spinner_enabled = not (utils.is_debug() or args.dry_run or args.quiet)

    # Execute the path transformation specification.
    with Halo(
        spinner="dots", color="cyan", stream=sys.stderr, enabled=spinner_enabled
    ) as sp:

        sp = utils.SpinnerWrapper(sp, save=log.getLogger().level <= log.INFO)

        # construct a source object for the input
        data = None
        if input_file is None:
            data = Source(None, SourceType.UnTyped)
        else:
            data = Source(Path(str(input_file)), SourceType.Path)

        # run all the stages
        for ed in path:
            sp.start_stage(f"{ed.stage.name} → {ed.stage.target_stage}")
            try:
                if ed.stage._no_spinner:
                    sp.stop()
                    result = ed.stage.run(data, None)
                else:
                    result = ed.stage.run(data, sp)
                data = result
                sp.end_stage()
            except errors.StepFailure as e:
                sp.fail()
                print(e)
                exit(-1)

        sp.stop()

        # output the data returned from the file step
        if args.output_file is not None:
            if data.typ == SourceType.Directory:
                shutil.move(data.data.name, args.output_file)
            else:
                with Path(args.output_file).open("wb") as f:
                    f.write(data.convert_to(SourceType.Bytes).data)
        elif data:
            print(data.convert_to(SourceType.String).data)
