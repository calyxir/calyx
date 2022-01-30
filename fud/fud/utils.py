import sys
import logging as log
import shutil
from tempfile import TemporaryDirectory, NamedTemporaryFile, TemporaryFile
from io import BytesIO, IOBase
from pathlib import Path
import subprocess
import os

from . import errors


def eprint(*args, **kwargs):
    print(*args, **kwargs, file=sys.stderr)


def is_warning():
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
    # Color for warning, error, and info messages.
    log.addLevelName(log.INFO, "\033[1;34m%s\033[1;0m" % log.getLevelName(log.INFO))
    log.addLevelName(
        log.WARNING, "\033[1;33m%s\033[1;0m" % log.getLevelName(log.WARNING)
    )
    log.addLevelName(log.ERROR, "\033[1;31m%s\033[1;0m" % log.getLevelName(log.ERROR))

    # Set verbosity level.
    level = None
    if "verbose" not in args or args.verbose == 0:
        level = log.WARNING
    elif args.verbose == 1:
        level = log.INFO
    elif args.verbose >= 2:
        level = log.DEBUG

    log.basicConfig(
        format="[fud] %(levelname)s: %(message)s", stream=sys.stderr, level=level
    )

    try:
        import paramiko

        paramiko.util.logging.getLogger().setLevel(level)
    except ModuleNotFoundError:
        pass


class Directory:
    """
    Represents a Directory path
    """

    def __init__(self, name):
        self.name = name

    def remove(self):
        shutil.rmtree(self.name)


class TmpDir(Directory):
    """A temporary directory that is automatically deleted."""

    def __init__(self):
        self.tmpdir_obj = TemporaryDirectory()
        self.name = self.tmpdir_obj.name

    def remove(self):
        self.tmpdir_obj.cleanup()

    def __str__(self):
        return self.name


class FreshDir(Directory):
    """A new empty directory for saving results into.

    The directory is created in the current working directory with an
    arbitrary name. This way, `FreshDir` works like `TmpDir` except the
    directory is not automatically removed. (It can still be manually
    deleted, of course.)
    """

    def __init__(self):
        # Select a name that doesn't exist.
        i = 0
        while True:
            name = "fud-out-{}".format(i)
            if not os.path.exists(name):
                break
            i += 1

        # Create the directory.
        os.mkdir(name)
        self.name = os.path.abspath(name)


class Conversions:
    @staticmethod
    def path_to_directory(data: Path):
        if data.is_dir():
            return Directory(data)
        else:
            raise errors.SourceConversionNotDirectory(data)

    @staticmethod
    def path_to_stream(data: Path):
        return open(data, "rb")

    @staticmethod
    def stream_to_path(data: IOBase) -> Path:
        with NamedTemporaryFile("wb", delete=False) as tmpfile:
            tmpfile.write(data.read())
            data.close()
            return Path(tmpfile.name)

    @staticmethod
    def stream_to_bytes(data: IOBase) -> bytes:
        out = data.read()
        data.close()
        return out

    @staticmethod
    def bytes_to_stream(data: bytes) -> IOBase:
        return BytesIO(data)

    @staticmethod
    def bytes_to_string(data: bytes) -> str:
        return data.decode("UTF-8")

    @staticmethod
    def string_to_bytes(data: str) -> bytes:
        return data.encode("UTF-8")


def shell(cmd, stdin=None, stdout_as_debug=False, capture_stdout=True):
    """Run `cmd` as a shell command.

    Return an output stream (or None if stdout is not captured). Raise
    `errors.StepFailure` if the command fails.
    """

    if isinstance(cmd, list):
        cmd = " ".join(cmd)

    if stdout_as_debug:
        cmd += ">&2"

    assert isinstance(cmd, str)
    log.debug(cmd)

    # In debug mode, let stderr stream to the terminal (and the same
    # with stdout, unless we need it for capture). Otherwise, capture
    # stderr to a temporary file for error reporting (and stdout
    # unconditionally).
    if is_debug():
        stderr = None
        if capture_stdout:
            stdout = TemporaryFile()
        else:
            stdout = None
    else:
        stderr = TemporaryFile()
        stdout = TemporaryFile()

    proc = subprocess.Popen(
        cmd,
        shell=True,
        stdin=stdin,
        stdout=stdout,
        stderr=stderr,
        env=os.environ,
    )
    proc.wait()
    if stdout:
        stdout.seek(0)

    if proc.returncode:
        if stderr:
            stderr.seek(0)
        raise errors.StepFailure(
            cmd,
            stdout.read().decode("UTF-8") if stdout else "No stdout captured.",
            stderr.read().decode("UTF-8") if stderr else "No stderr captured.",
        )

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

    proc = subprocess.Popen(cmd, env=os.environ, shell=True)

    proc.wait()


def parse_profiling_input(args):
    """
    Returns a mapping from stage to steps from the `profiled_stages` argument.
    For example, if the user passes in `-pr a.a1 a.a2 b.b1 c`, this will return:
    {"a" : ["a1", "a2"], "b" : ["b1"], "c" : [] }
    """
    stages = {}
    if args.profiled_stages is None:
        return stages

    for stage_step in args.profiled_stages:
        if "." in stage_step:
            stage, step = stage_step.split(".")
        else:
            stage, step = stage_step, None
        # If stage has not been added it, add it.
        if stage not in stages:
            stages[stage] = []
        if step is not None:
            stages[stage].append(step)

    return stages


def profiling_dump(stage, phases, durations) -> str:
    """
    Returns time elapsed during each stage or step of the fud execution.
    """

    def name_and_space(s: str) -> str:
        # Return a string containing `s` followed by max(32 - len(s), 1) spaces.
        return "".join((s, max(32 - len(s), 1) * " "))

    return f"{name_and_space(stage)}elapsed time (s)\n" + "\n".join(
        f"{name_and_space(p)}{round(t, 3)}" for p, t in zip(phases, durations)
    )


def profiling_csv(stage, phases, durations) -> str:
    """
    Dumps the profiling information into a CSV format.
    For example, with
        stage:     `x`
        phases:    ['a', 'b', 'c']
        durations: [1.42, 2.0, 3.4445]
    The output will be:
    ```
    x,a,1.42
    x,b,2.0
    x,c,3.444
    ```
    """
    return "\n".join(
        [f"{stage},{p},{round(t, 3)}" for (p, t) in zip(phases, durations)]
    )


def profile_stages(stage, phases, durations, is_csv) -> str:
    """
    Returns either a human-readable or CSV format profiling information,
    depending on `is_csv`.
    """
    kwargs = {
        "stage": stage,
        "phases": phases,
        "durations": durations,
    }
    return profiling_csv(**kwargs) if is_csv else profiling_dump(**kwargs)
