"""The definitions of fud stages."""

import functools
import logging as log
import os
import subprocess
from enum import Enum
from io import BytesIO
from tempfile import NamedTemporaryFile, TemporaryFile
from pathlib import Path

from .. import errors
from ..utils import is_debug, Directory


class SourceType(Enum):
    """
    Enum capturing the kind of source this is.
    TODO: document types
    TODO: replace untyped with custom named type
    """

    Path = 1
    Directory = 2
    Stream = 3
    String = 4
    Null = 5
    UnTyped = 6

    def __str__(self):
        if self == SourceType.Path:
            return "Path"
        elif self == SourceType.Directory:
            return "Directory"
        elif self == SourceType.Stream:
            return "Stream"
        elif self == SourceType.String:
            return "String"
        elif self == SourceType.Null:
            return "Null"
        elif self == SourceType.UnTyped:
            return "UnTyped"


class Source:
    def __init__(self, data, typ):
        self.typ = typ
        self.convert_map = {
            SourceType.Path: {
                SourceType.String: self._to_string,
                SourceType.Stream: self._to_stream,
                SourceType.Directory: self._to_directory,
            },
            SourceType.Directory: {},
            SourceType.Stream: {
                SourceType.String: self._to_string,
                SourceType.Path: self._to_path,
            },
            SourceType.String: {
                SourceType.Stream: self._to_stream,
            },
        }

        self.data = data

    def is_convertible_to(self, other):
        if self.typ == other:
            return True
        else:
            return other in self.convert_map[self.typ]

    def convert_to(self, other):
        if self.typ == other:
            return self

        if other in self.convert_map[self.typ]:
            self.convert_map[self.typ][other]()
            self.typ = other
            return self

        raise Exception(f"Can't convert from {self.typ} to {other}")

    def _to_path(self):
        if self.typ == SourceType.Stream:
            with NamedTemporaryFile("wb", delete=False) as tmpfile:
                tmpfile.write(self.data.read())
                self.data = tmpfile.name
        else:
            raise Exception("TODO")

    def _to_string(self):
        if self.typ == SourceType.Path:
            with open(self.data, "rb") as f:
                self.data = f.read().decode("UTF-8")
        elif self.typ == SourceType.Stream:
            self.data = self.data.read().decode("UTF-8")
        else:
            raise Exception("TODO")

    def _to_stream(self):
        if self.typ == SourceType.Path:
            self.data = open(self.data, "rb")
        elif self.typ == SourceType.String:
            self.data = BytesIO(self.data.encode("UTF-8"))

    def _to_directory(self):
        if self.typ == SourceType.Path:
            if Path(self.data).is_dir():
                self.data = Directory(self.data)
            else:
                raise errors.SourceConversionNotDirectory(self.data)
        else:
            raise Exception("TODO")

    def __repr__(self):
        return f"<Source {self.data} {self.typ} >"


class Stage:
    """
    Represents a stage in the execution pipeline. This encompasses
    the process of transforming one file type into the next.
    `name`: The name of this stage.
    `target_stage`: The name of the stage generated by this.
    `config`: The configuration object read from disk + any
              dynamic modifications made with `-s`.
    `description`: Description of this stage
    """

    def __init__(
        self, name, target_stage, input_type, output_type, config, description=None
    ):
        self.name = name
        self.target_stage = target_stage
        self.input_type = input_type
        self.output_type = output_type
        self.config = config
        if ["stages", self.name, "exec"] in self.config:
            self.cmd = self.config["stages", self.name, "exec"]
        else:
            self.cmd = None
        self.description = description
        self.steps = []

    def setup(self):
        self.hollow_input_data = Source(None, self.input_type)
        self.final_output = self._define_steps(self.hollow_input_data)

    def step(self, input_type=None, output_type=None, description=None):
        if input_type == SourceType.Null or input_type is None:
            input_type = ()
        elif type(input_type) != tuple:
            input_type = (input_type,)

        def step_decorator(function):
            functools.wraps(function)

            # the modified function that the decorator creates
            def wrapper(*args):
                # check to make sure the num of args match the num of expected args
                if len(args) != len(input_type):
                    raise Exception(
                        "Expected {} input arguments, but only recieved {}".format(
                            len(input_type), len(args)
                        )
                    )

                # make sure that the args are convertible to expected input types
                for arg, inp in zip(args, input_type):
                    if arg.typ != inp and not arg.is_convertible_to(inp):
                        raise Exception(
                            f"Type mismatch: can't convert {arg.typ} to {inp}"
                        )

                # create a source with no data so that we can return a handle to this
                future_output = Source(None, output_type)
                # convert the args to the right types and unwrap them
                unwrapped_args = map(
                    lambda a: a[0].convert_to(a[1]).data, zip(args, input_type)
                )
                # thunk the function as a Step
                self.steps.append(
                    Step(
                        function.__name__,
                        function,
                        unwrapped_args,
                        future_output,
                        description,
                    )
                )
                # return handle to the thing this function will return
                return future_output

            return wrapper

        return step_decorator

    def _define_steps(self, input_data):
        pass

    def run(self, input_data):
        assert isinstance(input_data, Source)

        # fill in input_data
        self.hollow_input_data.data = input_data.convert_to(self.input_type).data

        # run all the steps
        for step in self.steps:
            step()

        return self.final_output

    def dry_run(self):
        for i, step in enumerate(self.steps):
            print(f"  {i+1}) {step}")


class Step:
    def __init__(self, name, func, args, output, description):
        self.name = name
        self.func = func
        self.args = args
        self.output = output
        self.description = description

    def __call__(self):
        if is_debug():
            args = list(self.args)
            arg_str = ", ".join(map(lambda a: str(a), args))
            log.debug(f"{self.name}({arg_str})")
            self.args = args
        self.output.data = self.func(self, *self.args)
        return self.output

    def __str__(self):
        if self.description is not None:
            return f"{self.name}: {self.description}"
        elif self.func.__doc__ is not None:
            return f"{self.name}: {self.func.__doc__.strip()}"
        else:
            return f"{self.name}: <python function>"

    def shell(self, cmd, stdin=None, stdout_as_debug=False):
        """
        Runs `cmd` in the shell and returns a stream of the output.
        Raises `errors.StepFailure` if the command fails.
        """

        if isinstance(cmd, list):
            cmd = " ".join(cmd)

        if stdout_as_debug:
            cmd += ">&2"

        assert isinstance(cmd, str)

        self.description = cmd
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
        if proc.returncode != 0:
            stderr.seek(0)
            raise errors.StepFailure(stderr.read().decode("UTF-8"))
        stdout.seek(0)
        return stdout
