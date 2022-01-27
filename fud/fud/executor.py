import time


class DummySpinner:
    """
    Dummy class to implement the interface of a spinner object.
    All methods are a sham and do nothing.
    """

    def __init__(self):
        pass

    def start(self, text=None):
        pass

    def stop(self, text=None):
        pass

    def fail(self, text=None):
        pass

    def succeed(self, text=None):
        pass


class Profiler:
    """
    Interface for profiling runtime
    """

    def __init__(self):
        self._current_time = None

    def start(self):
        assert self._current_time is None, "Attempt to start multiple measurements"
        self._current_time = time.time()

    def end(self):
        assert (
            self._current_time is not None
        ), "Attempt to end measurement before it starts"
        t = self._current_time
        self._current_time = None
        return time.time() - t


class DummyProfiler:
    """
    Dummy profiler that does nothing
    """

    def __init__(self):
        pass

    def start(self):
        pass

    def end(self):
        pass


class Executor:
    """
    Executor for paths.
    """

    def __init__(self, spinner, persist=False, profile=False):
        # Persist outputs from the spinner
        self._persist = persist
        # Spinner object
        self._spinner = DummySpinner() if spinner is None else spinner
        # Profiler for this executor
        self._profiler = Profiler() if profile else DummyProfiler()

        # Current context
        self.ctx = []

        # Disable spinner outputs
        self._no_spinner = False
        # Mapping from stage -> step -> duration
        self.durations = {}

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        self._spinner.stop()

    # Control spinner behavior
    def disable_spinner(self):
        self._no_spinner = True

    def enable_spinner(self):
        self._no_spinner = False

    def stage(self, name, disable_spinner):
        """
        Use this to create a stage boundary:
            with executor.stage(name, disable_spinner):
                # Things to do with the stage
        """
        return StageExecutor(self, name, disable_spinner)

    def step(self, name):
        """
        Use this to create a step boundary:
            with executor.step(name):
                # Things to do with the step
        """
        assert self.ctx != [], "Attempt to create a step before a stage"
        return StepExecutor(self, name)

    def context(self, name, profile):
        return ContextExecutor(self, name, profile)

    def _update(self):
        if not self._no_spinner:
            msg = f"{'.'.join(self.ctx)}"
            self._spinner.start(msg)

    # Mark context boundaries
    def _start_ctx(self, name):
        self.ctx.append(name)

    def _end_ctx(self, is_err, profiling_data=None):
        msg = '.'.join(self.ctx)
        if profiling_data:
            msg += f" ({profiling_data} ms)"
        if self._persist:
            if is_err:
                self._spinner.fail(msg)
            else:
                self._spinner.succeed(msg)
        self.ctx.pop()
        self._update()

    # Mark stage boundaries. Use the stage method above instead of these.
    def _start_stage(self, name):
        self.ctx.append(name)
        self.durations[name] = {}

    def _end_stage(self, is_err):
        self.ctx.pop()

    # Mark step boundaries. Use the step method above instead of these.
    def _start_step(self, name):
        self.ctx.append(name)
        self._update()
        self._profiler.start()

    def _end_step(self, is_err):
        if self._persist:
            if is_err:
                self._spinner.fail()
            else:
                self._spinner.succeed()
        step = self.ctx.pop()
        self.durations[self.ctx[-1]][step] = self._profiler.end()
        self._update()

    def _stop(self):
        """
        Stops the spinner associated with this executor.
        """
        self._spinner.stop()


class StageExecutor(object):
    """
    Handles execution of a stage.
    """

    def __init__(self, parent_exec, stage, disable_spinner):
        self.parent_exec = parent_exec
        self.stage = stage
        if disable_spinner:
            self.parent_exec._stop()
            self.parent_exec.disable_spinner()

    def __enter__(self):
        self.parent_exec._start_stage(self.stage)

    def __exit__(self, exc_type, exc_value, traceback):
        self.parent_exec._end_stage(exc_type is not None)
        self.parent_exec.enable_spinner()


class StepExecutor(object):
    """
    Handles execution of a step.
    """

    def __init__(self, parent_exec, step):
        self.parent_exec = parent_exec
        self.step = step

    def __enter__(self):
        self.parent_exec._start_step(self.step)

    def __exit__(self, exc_type, exc_value, traceback):
        self.parent_exec._end_step(exc_type is not None)


class ContextExecutor(object):
    """
    Handles execution of a generic context.
    """

    def __init__(self, parent_exec, ctx, profile):
        self.parent_exec = parent_exec
        self.ctx = ctx
        self.profiler = Profiler() if profile else DummyProfiler()

    def __enter__(self):
        self.profiler.start()
        self.parent_exec._start_ctx(self.ctx)

    def __exit__(self, exc_type, exc_value, traceback):
        self.parent_exec._end_ctx(exc_type is not None, self.profiler.end())
