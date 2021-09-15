"""The definitions of fud stages."""

import functools
import inspect
import logging as log
from enum import Enum, auto
from io import IOBase
from pathlib import Path

from ..utils import Conversions as conv
from ..utils import Directory, is_debug


class SourceType(Enum):
    """
    Enum capturing the kind of source this is.
    TODO: replace untyped with custom named type
    @Path: Represents a local file path. Data is pathlib.Path.
    @Directory: Represents local directory. Data is utils.Directory.
    @Stream: Represents a file stream. Data is a file like object.
    @String: Represents a python string. Data is a string.
    @Bytes: Represents a python byte string. Data is bytes.
    @UnTyped: Represents anything. No guarantees on what data is.
    """

    Path = auto()
    Directory = auto()
    Stream = auto()
    String = auto()
    Bytes = auto()
    UnTyped = auto()

    def __str__(self):
        if self == SourceType.Path:
            return "Path"
        elif self == SourceType.Directory:
            return "Directory"
        elif self == SourceType.Stream:
            return "Stream"
        elif self == SourceType.String:
            return "String"
        elif self == SourceType.Bytes:
            return "Bytes"
        elif self == SourceType.UnTyped:
            return "UnTyped"


class Source:
    convert_map = {
        SourceType.Path: {
            SourceType.Directory: conv.path_to_directory,
            SourceType.Stream: conv.path_to_stream,
            SourceType.String: lambda p: conv.bytes_to_string(
                conv.stream_to_bytes(conv.path_to_stream(p))
            ),
            SourceType.Bytes: lambda p: conv.stream_to_bytes(conv.path_to_stream(p)),
        },
        SourceType.Stream: {
            SourceType.Path: conv.stream_to_path,
            SourceType.String: lambda s: conv.bytes_to_string(conv.stream_to_bytes(s)),
            SourceType.Bytes: conv.stream_to_bytes,
        },
        SourceType.String: {
            SourceType.Path: lambda s: conv.stream_to_path(
                conv.bytes_to_stream(conv.string_to_bytes(s))
            ),
            SourceType.Stream: lambda s: conv.bytes_to_stream(conv.stream_to_bytes(s)),
            SourceType.Bytes: conv.string_to_bytes,
        },
    }

    def __init__(self, data, typ):
        self.typ = typ
        # check to make sure data is the right type
        if data is not None:
            if self.typ == SourceType.Path:
                assert isinstance(data, Path)
            elif self.typ == SourceType.Directory:
                assert isinstance(data, Directory)
            elif self.typ == SourceType.Stream:
                assert isinstance(data, IOBase)
            elif self.typ == SourceType.String:
                assert isinstance(data, str)
            elif self.typ == SourceType.Bytes:
                assert isinstance(data, bytes)
            elif self.typ == SourceType.UnTyped:
                # no guarantees on Untyped
                pass
        self.data = data

    def is_convertible_to(self, other):
        if self.typ == other:
            return True
        else:
            return other in Source.convert_map[self.typ]

    def convert_to(self, other):
        if self.typ == other:
            return self

        if self.is_convertible_to(other):
            data = Source.convert_map[self.typ][other](self.data)
            return Source(data, other)

        raise Exception(f"Can't convert from {self.typ} to {other}")

    def __repr__(self):
        return f"<Source {self.data} {self.typ}>"


class Stage:
    """
    Represents a stage in the execution pipeline. This encompasses
    the process of transforming one file type into the next.
    `name`: The name of this stage.
    `target_stage`: The name of the stage generated by this.
    `input_type`: Type of the input stream accepted by this stage.
                  Must be a SourceType.
    `output_type`: Type of the output stream. Must be a SourceType.
    `config`: The configuration object read from disk + any
              dynamic modifications made with `-s`.
    `description`: Description of this stage
    """

    def __init__(
        self, name, target_stage, input_type, output_type, config, description
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
        self._no_spinner = False

    def setup(self):
        """
        Defines all the steps for this Stage by running self._define_steps.
        """
        self.hollow_input_data = Source(None, self.input_type)
        self.final_output = self._define_steps(self.hollow_input_data)

    def step(self, description=None):
        """
        Define a step for this Stage using a decorator.
        For example the following defines a step that runs a command in the
        shell:
            @self.step(description=self.cmd)
            def run_mrxl(mrxl_prog: SourceType.Path) -> SourceType.Stream:
                return shell(f"{self.cmd} {str(mrxl_prog)}")
        """

        def step_decorator(function):
            functools.wraps(function)

            sig = inspect.signature(function)

            annotations = []
            for ty in list(sig.parameters.values()):
                if ty.annotation is ty.empty:
                    raise Exception(f"Missing type annotation for argument `{ty}`")
                annotations.append(ty.annotation)
            input_types = tuple(annotations)

            # TODO: handle tuples return types
            output_types = sig.return_annotation

            # the modified function that the decorator creates
            def wrapper(*args):

                # check to make sure the num of args match the num of expected args
                if len(args) != len(input_types):
                    raise Exception(
                        "Expected {} input arguments, but only recieved {}".format(
                            len(input_types), len(args)
                        )
                    )

                # make sure that the args are convertible to expected input types
                for arg, inp in zip(args, input_types):
                    if arg.typ != inp and not arg.is_convertible_to(inp):
                        raise Exception(
                            f"Type mismatch: can't convert {arg.typ} to {inp}"
                        )

                # create a source with no data so that we can return a handle to this
                future_output = Source(None, output_types)
                # convert the args to the right types and unwrap them
                unwrapped_args = map(
                    lambda a: a[0].convert_to(a[1]).data, zip(args, input_types)
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

    def run(self, input_data, sp=None):
        assert isinstance(input_data, Source)

        # fill in input_data
        self.hollow_input_data.data = input_data.convert_to(self.input_type).data

        # run all the steps
        for step in self.steps:
            if sp is not None:
                sp.start_step(step.name)
            step()
            if sp is not None:
                sp.end_step()

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
        if description is not None:
            self.description = description
        elif self.func.__doc__ is not None:
            self.description = self.func.__doc__.strip()
        else:
            raise Exception(f"Step {self.name} does not have a description.")

    def __call__(self):
        if is_debug():
            args = list(self.args)
            arg_str = ", ".join(map(lambda a: str(a), args))
            log.debug(f"{self.name}({arg_str})")
            self.args = args
        self.output.data = self.func(*self.args)
        return self.output

    def __str__(self):
        if self.description is not None:
            return f"{self.name}: {self.description}"
        elif self.func.__doc__ is not None:
            return f"{self.name}: {self.func.__doc__.strip()}"
        else:
            return f"{self.name}: <python function>"
