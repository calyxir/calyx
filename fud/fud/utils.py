import sys
import logging
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
    else:
        return default

def logging_setup():
    # Color for warning and error mesages
    logging.addLevelName(
        logging.WARNING,
        "\033[1;33m%s\033[1;0m" % logging.getLevelName(logging.WARNING))
    logging.addLevelName(
        logging.ERROR,
        "\033[1;31m%s\033[1;0m" % logging.getLevelName(logging.ERROR))

    logging.basicConfig(
        format='%(levelname)s: %(message)s',
        stream=sys.stderr,
        level=logging.INFO)
