import sys
import logging as log


def eprint(*args, **kwargs):
    print(*args, **kwargs, file=sys.stderr)


def is_warming():
    return log.getLogger().level <= log.WARNING


def is_info():
    return log.getLogger().level <= log.INFO


def is_debug():
    return log.getLogger().level <= log.DEBUG


def unwrap_or(val, default):
    if val is not None:
        return val

    return default


def logging_setup(args):
    # Color for warning and error mesages
    log.addLevelName(
        log.WARNING,
        "\033[1;33m%s\033[1;0m" % log.getLevelName(log.WARNING))
    log.addLevelName(
        log.ERROR,
        "\033[1;31m%s\033[1;0m" % log.getLevelName(log.ERROR))

    # set verbosity level
    level = None
    if 'verbose' not in args or args.verbose == 0:
        level = log.WARNING
    elif args.verbose == 1:
        level = log.INFO
    elif args.verbose >= 2:
        level = log.DEBUG

    log.basicConfig(
        format='%(levelname)s: %(message)s',
        stream=sys.stderr,
        level=level
    )

    try:
        import paramiko
        paramiko.util.logging.getLogger().setLevel(level)
    except ModuleNotFoundError:
        pass
