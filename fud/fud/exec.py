from typing import List, Optional, Dict

import logging as log
import shutil
import sys
from pathlib import Path

from halo import Halo  # type: ignore

from . import errors, utils, executor
from .config import Configuration
from .stages import Source, SourceType, ComputationGraph, Stage


def gather_profiling_data(durations, stage, steps, is_csv):
    """
    Gather profiling information for a given stage.
    """
    data = durations.get(stage)
    # Verify this is a valid stage.
    if data is None:
        raise errors.UndefinedStage(stage, "Extracting profiling information")

    # If no specific steps provided for this stage, append all of them.
    if steps == []:
        profiled_steps = list(data.keys())
    else:
        # Verify the steps are valid.
        invalid_steps = [s for s in steps if s not in data.keys()]
        if invalid_steps:
            raise errors.UndefinedSteps(stage, invalid_steps, data.keys())
        profiled_steps = steps

    # Gather all the step names that are being profiled.
    profiled_durations = [data[s] for s in profiled_steps]
    return utils.profile_stages(
        stage,
        profiled_steps,
        profiled_durations,
        is_csv,
    )


def report_profiling(profiled_stages: Dict[str, List[str]], durations, is_csv):
    """
    Report profiling information collected during execution.
    """
    data = Source("", SourceType.String)

    if not profiled_stages:
        totals = []
        for stage, step_times in durations.items():
            totals.append(sum(step_times.values()))
        # No stages provided; collect overall stage durations.
        data.data = utils.profile_stages("stage", durations.keys(), totals, is_csv)
    else:
        # Otherwise, gather profiling data for each stage and steps provided.
        data.data = "\n".join(
            gather_profiling_data(durations, stage, steps, is_csv)
            for stage, steps in profiled_stages.items()
        )

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

    path = chain_stages(
        config.construct_path(
            args.source, args.dest, args.input_file, args.output_file, args.through
        ),
        config,
    )

    # check if input is needed
    if args.input_file is None:
        if path.input_type not in [SourceType.UnTyped, SourceType.Terminal]:
            raise errors.NeedInputSpecified(path[0])

    # check if we need `-o` specified
    if args.output_file is None:
        if path.output_type == SourceType.Directory:
            raise errors.NeedOutputSpecified(path[-1])

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
        input = Source(None, SourceType.UnTyped)
    else:
        input = Source.path(input_file)

    # Execute the generated path
    with exec:
        for step in path.get_steps(input):
            # Execute step within the stage
            with exec.context(step.name):
                step()

    # Stages to be profiled
    profiled_stages = utils.parse_profiling_input(args)

    # Report profiling information if flag was provided.
    if enable_profile:
        output = report_profiling(profiled_stages, exec.durations, args.csv)
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
