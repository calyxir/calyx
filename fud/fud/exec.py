import logging as log
import sys
from halo import Halo
from pathlib import Path
import shutil

from .stages import Source, SourceType
from . import errors, utils


def discover_implied_stage(filename, config, possible_dests=None):
    """
    Use the mapping from filename extensions to stages to figure out which
    stage was implied.
    """
    if filename is None:
        raise errors.NoInputFile(possible_dests)

    suffix = Path(filename).suffix
    for (name, stage) in config['stages'].items():
        if 'file_extensions' in stage:
            for ext in stage['file_extensions']:
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

    # update the stages config with arguments provided via cmdline
    if args.dynamic_config is not None:
        for key, value in args.dynamic_config:
            config[['stages'] + key.split('.')] = value

    # find source
    source = args.source
    if source is None:
        source = discover_implied_stage(args.input_file, config)

    # find target
    target = args.dest
    if target is None:
        target = discover_implied_stage(
            args.output_file,
            config,
            possible_dests=config.REGISTRY.nodes[source]
        )

    path = config.REGISTRY.make_path(source, target)
    if path is None:
        raise errors.NoPathFound(source, target)

    # If the path doesn't execute anything, it is probably an error.
    if len(path) == 0:
        raise errors.TrivialPath(source)

    # if we are doing a dry run, print out stages and exit
    if args.dry_run:
        print("fud will perform the following steps:")

    # Pretty spinner.
    spinner_enabled = not (utils.is_debug() or args.dry_run or args.quiet)
    # Execute the path transformation specification.
    with Halo(
            spinner='dots',
            color='cyan',
            stream=sys.stderr,
            enabled=spinner_enabled) as sp:

        if input_file is None:
            inp = Source(None, SourceType.Nothing)
        else:
            inp = Source(str(input_file), SourceType.Path)

        for i, ed in enumerate(path):
            sp.start(f"{ed.stage.name} → {ed.stage.target_stage}")
            (result, stderr, retcode) = ed.stage.transform(
                inp,
                dry_run=args.dry_run,
                last=i == (len(path) - 1)
            )
            inp = result

            if retcode == 0:
                if log.getLogger().level <= log.INFO:
                    sp.succeed()
            else:
                if log.getLogger().level <= log.INFO:
                    sp.fail()
                else:
                    sp.stop()
                log.error(stderr)
                exit(retcode)
        sp.stop()

        # return early when there's a dry run
        if args.dry_run:
            return

        if inp.source_type == SourceType.TmpDir:
            if args.output_file is not None:
                if Path(args.output_file).exists():
                    shutil.rmtree(args.output_file)
                shutil.move(inp.data.name, args.output_file)
            else:
                shutil.move(inp.data.name, ".")
                print(f"Moved {inp.data.name} here.")
        else:
            if args.output_file is not None:
                with Path(args.output_file).open('wb') as f:
                    f.write(inp.data.read())
            else:
                print(inp.data.read().decode('UTF-8'))
