from logging import log

from fud.stages import Stage, SourceType
from fud import errors

import numpy as np
import simplejson as sjson
from contextlib import redirect_stdout, redirect_stderr
import io


class HwExecutionStage(Stage):
    def __init__(self, config):
        super().__init__(
            "xclbin",
            "fpga",
            SourceType.Path,
            SourceType.String,
            config,
            "Run an xclbin on an fpga",
        )

        self.data_path = self.config["stages", self.target_stage, "data"]

        try:
            import pyopencl as cl

            self.cl = cl
        except ImportError:
            raise errors.RemoteLibsNotInstalled()

        self.setup()

    def _define_steps(self, input_data):
        @self.step()
        def run(xclbin: SourceType.Path) -> SourceType.String:
            """Run the xclbin with datafile"""

            if self.data_path is None:
                raise errors.MissingDynamicConfiguration("fpga.data")

            data = sjson.load(open(self.data_path), use_decimal=True)

            with redirect_stderr(io.StringIO()):
                with redirect_stdout(io.StringIO()):
                    ctx = self.cl.create_some_context(0)
                    dev = ctx.devices[0]
                    cmds = self.cl.CommandQueue(ctx, dev)
                    prg = self.cl.Program(ctx, [dev], [xclbin.open("rb").read()])

            with redirect_stderr(io.StringIO()):
                with redirect_stdout(io.StringIO()):
                    prg.build()

            buffers = {}
            for mem in data.keys():
                # allocate memory on the device
                buf = self.cl.Buffer(
                    ctx,
                    self.cl.mem_flags.READ_WRITE | self.cl.mem_flags.COPY_HOST_PTR,
                    hostbuf=np.array(data[mem]["data"]).astype(np.uint32),
                )
                # TODO: use real type information
                buffers[mem] = buf

            with redirect_stderr(io.StringIO()):
                with redirect_stdout(io.StringIO()):
                    prg.Toplevel(cmds, (1,), (1,), np.uint32(10000), *buffers.values())

            # read the result
            output = {}
            for name, buf in buffers.items():
                out_buf = np.zeros_like(data[name]["data"]).astype(np.uint32)
                self.cl.enqueue_copy(cmds, out_buf, buf)
                buf.release()
                output[name] = list(map(lambda x: int(x), out_buf))

            # cleanup
            del ctx

            return sjson.dumps(output, indent=2, use_decimal=True)

        res = run(input_data)
        return res
