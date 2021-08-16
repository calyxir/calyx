import sys
import logging as log
import shutil
from tempfile import TemporaryDirectory, NamedTemporaryFile, TemporaryFile
from io import BytesIO
from pathlib import Path
import subprocess
import os

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
    def stream_to_bytes(data):
        return data.read()

    @staticmethod
    def bytes_to_stream(data):
        return BytesIO(data)

    @staticmethod
    def bytes_to_string(data):
        return data.decode("UTF-8")

    @staticmethod
    def string_to_bytes(data):
        return data.encode("UTF-8")


class SpinnerWrapper:
    """
    Wraps a spinner object.
    """

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


def shell(cmd, stdin=None, stdout_as_debug=False):
    """
    Runs `cmd` in the shell and returns a stream of the output.
    Raises `errors.StepFailure` if the command fails.
    """

    if isinstance(cmd, list):
        cmd = " ".join(cmd)

    if stdout_as_debug:
        cmd += ">&2"

    assert isinstance(cmd, str)

    log.debug(cmd)

    stdout = TemporaryFile()
    stderr = None
    # if we are not in debug mode, capture stderr
    if not is_debug():
        stderr = TemporaryFile()

    proc = subprocess.Popen(
        cmd, shell=True, stdin=stdin, stdout=stdout, stderr=stderr, env=os.environ
    )
    proc.wait()
    stdout.seek(0)
    if proc.returncode != 0:
        if stderr is not None:
            stderr.seek(0)
            raise errors.StepFailure(
                cmd, stdout.read().decode("UTF-8"), stderr.read().decode("UTF-8")
            )
        else:
            raise errors.StepFailure(cmd, "No stdout captured.", "No stderr captured.")
    return stdout


def transparent_shell(cmd):
    """
    Runs `cmd` in the shell. Does not capture output or input. Does nothing
    fancy and returns nothing
    """
    if isinstance(cmd, list):
        cmd = " ".join(cmd)

    assert isinstance(cmd, str)

    log.debug(cmd)

    proc = subprocess.Popen(cmd, env=os.environ)

    proc.wait()
