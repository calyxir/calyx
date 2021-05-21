import logging as log
import os
import time

import numpy as np
import simplejson as sjson

from fud import errors
from fud.stages import SourceType, Stage
from fud.utils import TmpDir


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

        self.setup()

    def _define_steps(self, input_data):
        @self.step()
        def import_libs():
            """Import optional libraries"""
            try:
                import pyopencl as cl

                self.cl = cl
            except ImportError:
                raise errors.RemoteLibsNotInstalled

        @self.step()
        def run(xclbin: SourceType.Path) -> SourceType.String:
            """Run the xclbin with datafile"""

            if self.data_path is None:
                raise errors.MissingDynamicConfiguration("fpga.data")

            data = sjson.load(open(self.data_path), use_decimal=True)
            xclbin_source = xclbin.open("rb").read()

            # create a temporary directory with an xrt.ini file that redirects
            # the runtime log to a file so that we can control how it's printed.
            # This is hacky, but it's the only way to do it
            tmp_dir = TmpDir()
            os.chdir(tmp_dir.name)
            xrt_output_logname = "output.log"
            with open("xrt.ini", "w+") as f:
                f.writelines(["[Runtime]\n", f"runtime_log={xrt_output_logname}"])

            ctx = self.cl.create_some_context(0)
            dev = ctx.devices[0]
            cmds = self.cl.CommandQueue(ctx, dev)
            prg = self.cl.Program(ctx, [dev], [xclbin_source])

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

            start_time = time.time()
            prg.Toplevel(cmds, (1,), (1,), np.uint32(10000), *buffers.values())
            end_time = time.time()

            # read the result
            output = {"memories": {}, "runtime": end_time - start_time}
            for name, buf in buffers.items():
                out_buf = np.zeros_like(data[name]["data"]).astype(np.uint32)
                self.cl.enqueue_copy(cmds, out_buf, buf)
                buf.release()
                output["memories"][name] = list(map(lambda x: int(x), out_buf))

            # cleanup
            del ctx
            # add xrt log output to our debug output
            with open(xrt_output_logname, "r") as f:
                for line in f.readlines():
                    log.debug(line.strip())

            return sjson.dumps(output, indent=2, use_decimal=True)

        import_libs()
        res = run(input_data)
        return res
