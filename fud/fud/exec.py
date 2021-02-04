import logging as log
import shutil
import sys
from pathlib import Path

from halo import Halo

from . import errors, utils
from .stages import Source, SourceType


class SpinnerWrapper:
    def __init__(self, spinner, save):
        self.spinner = spinner
        self.save = save
        self.stage_text = ""
        self.step_text = ""

    def _update(self):
        if self.step_text != "":
            self.spinner.start(f"{self.stage_text}: {self.step_text}")
        else:
            self.spinner.start(f"{self.stage_text}")

    def start_stage(self, text):
        self.stage_text = text
        self._update()

    def end_stage(self):
        if self.save:
            self.spinner.succeed()

    def start_step(self, text):
        self.step_text = text
        self._update()

    def end_step(self):
        if self.save:
            self.spinner.succeed()
        self.step_text = ""
        self._update()

    def succeed(self):
        self.spinner.succeed()

    def fail(self, text=None):
        self.spinner.fail(text)

    def stop(self):
        self.spinner.stop()


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


def run_fud(args, config):
    # check if input_file exists
    input_file = None
    if args.input_file is not None:
        input_file = Path(args.input_file)
        if not input_file.exists():
            raise FileNotFoundError(input_file)

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

    # Pretty spinner.
    spinner_enabled = not (utils.is_debug() or args.dry_run or args.quiet)
    # Execute the path transformation specification.
    with Halo(
        spinner="dots", color="cyan", stream=sys.stderr, enabled=spinner_enabled
    ) as sp:

        sp = SpinnerWrapper(sp, save=log.getLogger().level <= log.INFO)

        # if input_file is None:
        #     inp = Source(None, SourceType.Passthrough)
        # else:
        inp = Source(Path(str(input_file)), SourceType.Path)

        for ed in path:
            sp.start_stage(f"{ed.stage.name} → {ed.stage.target_stage}")
            # TODO: catch exceptions
            try:
                result = ed.stage.run(inp, sp)
                inp = result
                sp.end_stage()
            except errors.StepFailure as e:
                sp.fail()
                print(e)
                exit(-1)

        sp.stop()

        if args.output_file is not None:
            if inp.typ == SourceType.Directory:
                shutil.move(inp.data.name, args.output_file)
            else:
                with Path(args.output_file).open("w") as f:
                    f.write(inp.convert_to(SourceType.String).data)
        else:
            print(inp.convert_to(SourceType.String).data)
