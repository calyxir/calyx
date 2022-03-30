from typing import List, Optional, Dict

import logging as log
import shutil
import sys
from pathlib import Path

from halo import Halo  # type: ignore

from . import errors, utils, executor
from .config import Configuration
from .stages import Source, SourceType, ComputationGraph, Stage


def report_profiling(durations: Dict[str, float], is_csv: bool):
    """
    Report profiling information collected during execution.
    """
    data = Source(None, SourceType.String)
    data.data = utils.profile_stages(durations, is_csv)
    return data


def chain_stages(
    path: List[Stage], config: Configuration, builder: Optional[ComputationGraph] = None
) -> ComputationGraph:
    """
    Transform a path into a staged computation
    """
    assert len(path) > 0, "Path is empty"
    if builder is None:
        builder = path[0].setup(config)
    else:
        path[0].setup(config, builder)

    builder.and_then_path(path[1:], config)

    return builder


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

    path = config.construct_path(
        args.source, args.dest, args.input_file, args.output_file, args.through
    )

    # check if input is needed
    inp_type = path[0].input_type
    if args.input_file is None:
        if inp_type not in [SourceType.UnTyped, SourceType.Terminal, SourceType.Stream]:
            raise errors.NeedInputSpecified(path[0])

    # check if we need `-o` specified
    if args.output_file is None:
        if path[-1].output_type == SourceType.Directory:
            raise errors.NeedOutputSpecified(path[-1])

    path = chain_stages(path, config)

    # if we are doing a dry run, print out stages and exit
    if args.dry_run:
        print("fud will perform the following steps:")
        path.dry_run()
        return

    # spinner is disabled if we are in debug mode, doing a dry_run, or are in quiet mode
    spinner_enabled = not (utils.is_debug() or args.quiet)
    # Execute the path transformation specification.
    if spinner_enabled:
        sp = Halo(
            spinner="dots", color="cyan", stream=sys.stderr, enabled=spinner_enabled
        )
    else:
        sp = None

    enable_profile = args.profiled_stages is not None
    exec = executor.Executor(sp, log.getLogger().level <= log.INFO, enable_profile)

    # construct a source object for the input
    input = None
    if input_file is None:
        if inp_type is SourceType.Stream:
            input = Source(sys.stdin, SourceType.Stream)
        else:
            input = Source(None, SourceType.UnTyped)
    else:
        input = Source.path(input_file)

    # Execute the generated path
    with exec:
        for step in path.get_steps(input):
            # Execute step within the stage
            with exec.context(step.name):
                step()

    # Report profiling information if flag was provided.
    if enable_profile:
        output = Source(None, SourceType.String)
        if args.profiled_stages:
            durations = dict(
                filter(lambda kv: kv[0] in args.profiled_stages, exec.durations.items())
            )
        else:
            durations = exec.durations
        output.data = utils.profile_stages(durations, args.csv)
    else:
        output = path.output

    # output the data or profiling information.
    if args.output_file is not None:
        if output.typ == SourceType.Directory:
            shutil.move(output.data.name, args.output_file)
        else:
            with Path(args.output_file).open("wb") as f:
                f.write(output.convert_to(SourceType.Bytes).data)
    elif output:
        print(output.convert_to(SourceType.String).data)
