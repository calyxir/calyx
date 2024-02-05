class FudError(Exception):
    """
    An error caught by the Calyx Driver.
    """


class DeprecatedState(FudError):
    """
    The given state has been deprecated
    """

    def __init__(self, stage, state, alt=None):
        msg = (
            f"Stage `{stage}' acts upon deprecated state `{state}'"
            f". Use state `{alt}' instead in the stage definition"
            if alt
            else ""
        )
        super().__init__(msg)


class CycleLimitedReached(FudError):
    """
    The cycle limit has been reached for simulation.
    """

    def __init__(self, stage, cycle_limit):
        super().__init__(
            f"The cycle limit for simulation: {cycle_limit} "
            "has been reached. Either your program is not making progress, "
            "or you need to increase the cycle limit with the flag: "
            f"\n    -s {stage}.cycle_limit <cycle-limit>"
        )


class UnknownExtension(FudError):
    """
    The provided extension does not correspond to any known stage.
    Thrown when the implicit stage discovery mechanism fails.
    """

    def __init__(self, msg, filename):
        super().__init__(msg + "Please provide an explicit stage using --to or --from.")


class UnsetConfiguration(FudError):
    """
    Execution of a stage requires some configuration.
    Thrown when missing configuration.
    """

    def __init__(self, path):
        path_str = ".".join(path)
        msg = (
            f"'{path_str}' is not set. "
            + f"Use `fud config {path_str} <val>` to set it."
        )
        super().__init__(msg)


class UnknownConfiguration(FudError):
    """
    Unknown configuration option provided to stage
    """

    def __init__(self, stage, opt, known):
        msg = (
            f"Unknown configuration option `{opt}' for stage `{stage}'. "
            + f"Known options: {', '.join(known)}"
        )
        super().__init__(msg)


class MissingDynamicConfiguration(FudError):
    """
    Execution of a stage requires dynamic configuration. Thrown when such a
    configuration is missing.
    """

    def __init__(self, variable):
        msg = (
            f"`{variable}' needs to be set. "
            + "Use the runtime configuration flag to provide a value: "
            + f"'-s {variable} <value>'."
        )
        super().__init__(msg)


class NoPathFound(FudError):
    """
    There is no way to convert file in input stage to the given output stage
    that go through the given stages.
    """

    def __init__(self, source, destination, through):
        msg = (
            f"No way to convert input in stage `{source}' to "
            + f"stage `{destination}' "
            + (
                f"that go through stage(s) {', '.join(through)}"
                if len(through) > 0
                else ""
            )
        )
        super().__init__(msg)


class UndefinedState(FudError):
    """
    No state with the defined name.
    """

    def __init__(self, stage, ctx=None):
        msg = f"No state named {stage}"
        if ctx is not None:
            msg += f". Context: {ctx}"
        super().__init__(msg)


class UndefinedSteps(FudError):
    """
    No steps with the defined name for the given stage.
    """

    def __init__(self, stage, steps, known_steps):
        msg = f"No step(s): {', '.join(steps)} defined for stage: {stage}"
        if known_steps is not None:
            msg += f". Known steps: {', '.join(known_steps)}"
        super().__init__(msg)


class MultiplePaths(FudError):
    """
    Multiple paths found to transform `src` to `dst`.
    """

    def __init__(self, src, dst, paths):
        msg = (
            f"Multiple stage pipelines can transform {src} to {dst}:\n"
            + paths
            + "\nUse the --through flag to select an intermediate stage."
            + " See https://docs.calyxir.org/fud/multiple-paths.html for"
            + " more information."
        )
        super().__init__(msg)


class TrivialPath(FudError):
    """
    The execution doesn't run any stages. Likely a user mistake.
    """

    def __init__(self, stage):
        msg = (
            f"The exection starts and ends at the same stage `{stage}'. "
            + "This is likely an error."
        )
        super().__init__(msg)


class SourceConversion(FudError):
    """
    Can't convert to a particular source type.
    """

    def __init__(self, output_type, why=None):
        msg = (
            f"Data cannot be converted into {output_type}. "
            "If an output stage produced it, "
            "provide name for an output file using the `-o` flag."
        )
        super().__init__(msg)


class RemoteLibsNotInstalled(FudError):
    """
    Libraries needed for remote use of tools are not installed.
    """

    def __init__(self):
        msg = (
            "Attempted to use remote features without "
            + "[paramiko, scp, pyopencl] installed. Install them and try again."
        )
        super().__init__(msg)


class MissingFile(FudError):
    """
    A stage expected a file to exist that didn't exist.
    """

    def __init__(self, filename):
        msg = (
            f"File doesn't exist: '{filename}'. "
            + "Check tool versions with `fud check`."
        )
        super().__init__(msg)


class StepFailure(FudError):
    """
    Indicates that a step failed.
    """

    def __init__(self, step, stdout, stderr):
        msg = (
            f"`{step.strip()}' failed:\n=====STDERR=====\n"
            + stderr
            + "\n=====STDOUT=====\n"
            + stdout
        )
        super().__init__(msg)


class NeedInputSpecified(FudError):
    """
    Error raised when the starting stage needs an input
    """

    def __init__(self, stage):
        msg = (
            f"The starting stage `{stage.name}` requires an input of type"
            f" `{stage.input_type}` but no input was provided."
        )
        super().__init__(msg)


class NeedOutputSpecified(FudError):
    """
    An error raised when the last stage will produce output that
    is not serializable as a text stream (i.e. a Directory)
    """

    def __init__(self, final_stage):
        msg = (
            f"The final stage: `{final_stage.name}` will produce a "
            + f"`{final_stage.output_type}` which can't be printed in the terminal. "
            + "Supply `-o <name>` to specify a name for it."
        )
        super().__init__(msg)


class SourceConversionNotDirectory(FudError):
    """
    An error raised when the last stage will produce output that
    is not serializable as a text stream (i.e. a Directory)
    """

    def __init__(self, path):
        msg = (
            f"Tried to convert {path} to a SourceType.Directory, "
            + "but it is not a directory."
        )
        super().__init__(msg)


class Malformed(FudError):
    """
    An error raised when the input to a stage is malformed in some manner.
    """

    def __init__(self, name, msg):
        msg = f"""Malformed {name}: {msg}"""
        super().__init__(msg)


class InvalidExternalStage(FudError):
    """
    An error raised when an external stage is not valid.
    """

    def __init__(self, stage_name, msg):
        msg = f"""Unable to load external stage: {stage_name}
{msg}"""
        super().__init__(msg)


class FudRegisterError(FudError):
    """
    An error raised when an external stage is not valid.
    """

    def __init__(self, conf, msg):
        msg = f"""Failed to register `{conf}': {msg}"""
        super().__init__(msg)
