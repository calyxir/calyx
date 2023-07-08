from __future__ import annotations
from typing import TYPE_CHECKING, List, Optional, Union, Any, Dict, Callable, Iterable

"""The definitions of fud stages."""
if TYPE_CHECKING:
    from .config import Configuration

import functools
import inspect
import logging as log
from enum import Enum, auto
from io import IOBase
from pathlib import Path

from ..utils import Conversions as conv
from ..utils import Directory, is_debug


class Step:
    """
    A Step represents some delayed computation that is a part of a stage.
    They are generally created using the @step decorator.
    """

    def __init__(
        self, name: str, func, args: Iterable[Any], output: Source, description: str
    ):
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

        # Whether this Step has been executed or not.
        self.executed = False

    def __call__(self):
        assert not self.executed, "Attempting to re-execute the same step"

        if is_debug():
            args = list(self.args)
            arg_str = ", ".join(map(lambda a: str(a), args))
            log.debug(f"{self.name}({arg_str})")
            self.args = args
        self.output.data = self.func(*self.args)
        self.executed = True
        return self.output

    def __str__(self):
        return f"{self.name}: {self.description}"


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
    @Terminal: Source will not return and `fud` should hand off control.
    """

    Path = auto()
    Directory = auto()
    Stream = auto()
    String = auto()
    Bytes = auto()
    UnTyped = auto()
    Terminal = auto()

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
        elif self == SourceType.Terminal:
            return "Terminal"


class Source:
    convert_map: Dict[SourceType, Dict[SourceType, Callable[[Any], Any]]] = {
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
        SourceType.Directory: {
            SourceType.String: lambda d: d.name,
            SourceType.Path: lambda d: Path(d.name),
        },
        # Terminal and UnTyped cannot be converted
        SourceType.Terminal: {},
        SourceType.UnTyped: {},
    }

    @staticmethod
    def path(path: Optional[Union[str, Path]] = None) -> Source:
        if path is not None and isinstance(path, str):
            path = Path(str(path))
        return Source(path, SourceType.Path)

    def __init__(self, data: Optional[Any], typ: SourceType):
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
        if self.typ == SourceType.Terminal:
            assert data is None, "Terminal Source cannot contain data"
        self.data = data

    def is_convertible_to(self, other: SourceType):
        if self.typ == other:
            return True
        else:
            return other in Source.convert_map[self.typ]

    def convert_to(self, other: SourceType) -> Source:
        assert self.typ != SourceType.Terminal, "Terminal data cannot be converted"
        assert self.typ == SourceType.UnTyped or self.data is not None

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
    `src_stage`: Name of the starting state.
    `target_stage`: The name of the state generated by this.
    `input_type`: Type of the input stream accepted by this stage.
                  Must be a SourceType.
    `output_type`: Type of the output stream. Must be a SourceType.
    `config`: The configuration object read from disk + any
              dynamic modifications made with `-s`.
    `description`: Description of this stage
    """

    # The name of a Stage is shared by all instances of the stage.
    name = ""

    def __init__(
        self,
        *,  # Force naming of the arguments
        src_state: str,
        target_state: str,
        input_type: SourceType,
        output_type: SourceType,
        description: str,
    ):
        self.src_state = src_state
        self.target_state = target_state
        self.input_type = input_type
        self.output_type = output_type

        self.description = description

    def known_opts(self) -> Optional[List[str]]:
        """
        Return a list of known options for this stage.
        If None, we don't know what options are available.
        """
        return None

    def _check_opts(self, config: Configuration):
        """
        Check that the options provided by the user are valid.
        """
        known = self.known_opts()
        if known:
            # Get all the options defined for this stage
            opts = config["stages", self.name]
            for opt in opts.keys():
                if opt not in known:
                    log.warn(
                        f"Unknown option `{self.name}.{opt}' for stage `{self.name}'"
                    )

    def setup(
        self,
        config: Configuration,
        builder: Optional[ComputationGraph] = None,
    ) -> ComputationGraph:
        """
        Construct a computation graph for this stage.
        Returns a `ComputationGraph` representing the staged computation.
        """

        # Check that the options provided by the user are valid.
        self._check_opts(config)

        # If a builder is provided, construct the computation graph using it.
        if builder:
            # Builder's current output because the stage's input.
            builder.and_then(self, config)
        else:
            builder = ComputationGraph(self.input_type, self.output_type)
            builder.ctx.append(self.name)
            builder.output = self._define_steps(builder._input, builder, config)
            builder.ctx.pop()

        return builder

    def _define_steps(
        self, input: Source, builder: ComputationGraph, config: Configuration
    ) -> Source:
        """
        Generate the staged execution graph for this Stage. Generally, this
        function will define all the steps in this Stage and define an execution
        schedule for those stages.
        When executed, each step will be added to this Stage's computation
        graph.
        """
        pass


class ComputationGraph:

    """Construct the computation graph for a stage"""

    def __init__(
        self,
        input_type: SourceType,
        output_type: SourceType,
    ):
        self.input_type = input_type
        self.output_type = output_type

        # Steps defined for this execution graph.
        self.steps: List[Step] = []
        # Input this computation graph
        self._input = Source(None, self.input_type)

        # Current context. Used to providing better stage names.
        self.ctx: List[str] = []

        self.output: Optional[Source] = None

    def dry_run(self):
        """
        Print out step information without running them.
        """
        for step in self.steps:
            print(f"  - {step}")

    def and_then(self, stage: Stage, config: Configuration):
        """
        Compose the stage's computation graph with the current graph.
        The steps in the stage will execute after this computation graph and
        will take the final output as their input.
        """
        assert self.output is not None

        # If the stage's input type doesn't match the current output type,
        # convert it.
        if self.output.typ != stage.input_type:
            input = self.convert_source_to(self.output, stage.input_type)
        else:
            input = self.output

        self.ctx.append(stage.name)
        self.output = stage._define_steps(input, self, config)
        self.ctx.pop()
        self.output_type = stage.output_type
        return self

    def also_do(self, input: Source, stage: Stage, config: Configuration) -> Source:
        """
        Define a branch of the computation graph that may use any input.
        """
        return stage._define_steps(input, self, config)

    def and_then_path(self, path: List[Stage], config: Configuration):
        """
        Convienience method to stage all the computations in a path.
        """
        for stage in path:
            stage.setup(config, self)

    def also_do_path(
        self, input: Source, path: List[Stage], config: Configuration
    ) -> Source:
        """
        A branch of the computation graph that uses `input` and executes the
        given path.
        @returns The output generated by this the branch of computation
        """
        assert path is not None and len(path) > 0, "Path is empty"
        first = path[0]
        # The first stage uses the input to this computation graph
        out = self.also_do(input, first, config)
        # The remaining path is connected to the other builder
        for stage in path[1:]:
            out = self.also_do(out, stage, config)

        return out

    def get_steps(self, input_data: Source):
        """
        Steps associated with this computation graph
        """
        self._input.data = input_data.convert_to(self.input_type).data

        for step in self.steps:
            yield step

    def convert_source_to(self, input: Source, output_type: SourceType) -> Source:
        """
        Returns a Step that converts input to the `output_type` SourceType
        """

        def transform_source(input: Source) -> Any:
            return input.convert_to(output_type).data

        output = Source(None, output_type)
        convert_step = Step(
            "transform",
            transform_source,
            [input],
            output,
            f"transform input to {output_type}",
        )
        self.steps.append(convert_step)
        return output

    def step(builder: ComputationGraph, description=None):
        """
        Define a step for this Stage using a decorator.
        For example the following defines a step that runs a command in the
        shell:
            @builder.step(description=cmd)
            def run_mrxl(mrxl_prog: SourceType.Path) -> SourceType.Stream:
                return shell(f"{cmd} {str(mrxl_prog)}")
        """

        # Define a function because the decorator needs to take in arguments.
        def step_decorator(function):
            """
            Decorator that transforms functions into `Step` and ensures that
            the input and output type match.
            """
            functools.wraps(function)

            sig = inspect.signature(function)

            annotations = []
            for ty in list(sig.parameters.values()):
                if ty.annotation is ty.empty:
                    raise Exception(
                        f"Missing type annotation for argument `{ty}`."
                        " Steps require `SourceType` types for all arguments"
                    )
                annotations.append(ty.annotation)
            input_types: List[SourceType] = tuple(annotations)

            # TODO: handle tuples return types
            output_types = sig.return_annotation

            # the modified function that the decorator creates
            def wrapper(*args: Source) -> Source:
                # check to make sure the num of args match the num of expected
                # args
                if len(args) != len(input_types):
                    raise Exception(
                        f"Expected {len(input_types)} input arguments,"
                        f" but recieved {len(args)}"
                    )

                # make sure that the args are convertible to expected input
                # types
                for arg, inp in zip(args, input_types):
                    assert isinstance(
                        arg, Source
                    ), f"Argument type is not source: ${type(arg)}"
                    if arg.typ != inp and not arg.is_convertible_to(inp):
                        raise Exception(
                            f"Type mismatch: can't convert {arg.typ} to {inp}"
                        )

                # Create a source with no data so that we can return a handle
                # to this.
                # When this step executes, this is updated to contain the data
                # generated by the step.
                future_output = Source(None, output_types)

                # convert the args to the right types and unwrap them
                # NOTE(rachit): This is a *LAZY* computaion and only occurs when
                # the step's data has been filled.
                unwrapped_args = map(
                    lambda a: a[0].convert_to(a[1]).data, zip(args, input_types)
                )
                if builder.ctx:
                    name = f"{'.'.join(builder.ctx)}.{function.__name__}"
                else:
                    name = function.__name__
                # thunk the function as a Step and add it to the current stage.
                builder.steps.append(
                    Step(
                        name,
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
