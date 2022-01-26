import logging as log
import shutil
import sys
from pathlib import Path

from halo import Halo

from . import errors, utils, executor, stages
from .stages import Source, SourceType


def construct_path(
    config, source=None, target=None, input_file=None, output_file=None, through=[]
) -> list[stages.Stage]:
    """
    Construct the path of stages implied by the passed arguments.
    """
    # find source
    if source is None:
        source = config.discover_implied_states(input_file)

    # find target
    if target is None:
        target = config.discover_implied_states(output_file)

    path = config.registry.make_path(source, target, through)
    if path is None:
        raise errors.NoPathFound(source, target, through)

    # If the path doesn't execute anything, it is probably an error.
    if len(path) == 0:
        raise errors.TrivialPath(source)

    return path


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


def report_profiling(profiled_stages, durations, is_csv):
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

    path = construct_path(
        config, args.source, args.dest, args.input_file, args.output_file, args.through
    )

    # check if we need `-o` specified
    if path[-1].output_type == SourceType.Directory and args.output_file is None:
        raise errors.NeedOutputSpecified(path[-1])

    # if we are doing a dry run, print out stages and exit
    if args.dry_run:
        print("fud will perform the following steps:")
        for ed in path:
            print(f"Stage: {ed.name}")
            ed.dry_run()
        return

    # construct a source object for the input
    data = None
    if input_file is None:
        data = Source(None, SourceType.UnTyped)
    else:
        data = Source(Path(str(input_file)), SourceType.Path)

    # spinner is disabled if we are in debug mode, doing a dry_run, or are in quiet mode
    spinner_enabled = not (utils.is_debug() or args.dry_run or args.quiet)
    # Execute the path transformation specification.
    if spinner_enabled:
        sp = Halo(
            spinner="dots", color="cyan", stream=sys.stderr, enabled=spinner_enabled
        )
    else:
        sp = None

    enable_profile = args.profiled_stages is not None
    exec = executor.Executor(sp, log.getLogger().level <= log.INFO, enable_profile)
    # Execute the generated path
    with exec:
        for ed in path:
            # Start a new stage
            with exec.stage(ed.name, ed._no_spinner):
                for step in ed.get_steps(data):
                    # Execute step within the stage
                    with exec.step(step.name):
                        step()

                data = ed.output()

    # Stages to be profiled
    profiled_stages = utils.parse_profiling_input(args)

    # Report profiling information if flag was provided.
    if enable_profile:
        data = report_profiling(profiled_stages, exec.durations, args.csv)

    # output the data or profiling information.
    if args.output_file is not None:
        if data.typ == SourceType.Directory:
            shutil.move(data.data.name, args.output_file)
        else:
            with Path(args.output_file).open("wb") as f:
                f.write(data.convert_to(SourceType.Bytes).data)
    elif data:
        print(data.convert_to(SourceType.String).data)
