import sys
import logging as log
import shutil
from tempfile import TemporaryDirectory, NamedTemporaryFile
from io import BytesIO
from pathlib import Path

from . import errors


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
        log.WARNING, "\033[1;33m%s\033[1;0m" % log.getLevelName(log.WARNING)
    )
    log.addLevelName(log.ERROR, "\033[1;31m%s\033[1;0m" % log.getLevelName(log.ERROR))

    # set verbosity level
    level = None
    if "verbose" not in args or args.verbose == 0:
        level = log.WARNING
    elif args.verbose == 1:
        level = log.INFO
    elif args.verbose >= 2:
        level = log.DEBUG

    log.basicConfig(format="%(levelname)s: %(message)s", stream=sys.stderr, level=level)

    try:
        import paramiko

        paramiko.util.logging.getLogger().setLevel(level)
    except ModuleNotFoundError:
        pass


class Directory:
    def __init__(self, name):
        self.name = name

    def remove(self):
        shutil.rmtree(self.name)


class TmpDir(Directory):
    def __init__(self):
        self.tmpdir_obj = TemporaryDirectory()
        self.name = self.tmpdir_obj.name

    def remove(self):
        self.tmpdir_obj.cleanup()


class Conversions:
    @staticmethod
    def path_to_directory(data):
        if data.is_dir():
            return Directory(data.name)
        else:
            raise errors.SourceConversionNotDirectory(data.name)

    @staticmethod
    def path_to_stream(data):
        return open(data, "rb")

    @staticmethod
    def stream_to_path(data):
        with NamedTemporaryFile("wb", delete=False) as tmpfile:
            tmpfile.write(data.read())
            return Path(tmpfile.name)

    @staticmethod
    def stream_to_string(data):
        return data.read().decode("UTF-8")

    @staticmethod
    def string_to_stream(data):
        return BytesIO(data.encode("UTF-8"))
