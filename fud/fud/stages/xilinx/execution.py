import logging as log
import os
import time

import numpy as np
import simplejson as sjson

from fud import errors
from fud.stages import SourceType, Stage
from fud.utils import TmpDir


class HwExecutionStage(Stage):
    name = "fpga"

    def __init__(self):
        super().__init__(
            src_state="xclbin",
            target_state="fpga",
            input_type=SourceType.Path,
            output_type=SourceType.String,
            description="Run an xclbin on an fpga",
        )

    def _define_steps(self, input_data, config):
        data_path = config["stages", self.name, "data"]

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

            if data_path is None:
                raise errors.MissingDynamicConfiguration("fpga.data")

            data = sjson.load(open(data_path), use_decimal=True)
            xclbin_source = xclbin.open("rb").read()

            # create a temporary directory with an xrt.ini file that redirects
            # the runtime log to a file so that we can control how it's printed.
            # This is hacky, but it's the only way to do it
            tmp_dir = TmpDir()
            os.chdir(tmp_dir.name)
            xrt_output_logname = "output.log"
            with open("xrt.ini", "w") as f:
                f.writelines(
                    [
                        "[Runtime]\n",
                        f"runtime_log={xrt_output_logname}\n",
                        "[Emulation]\n",
                        "print_infos_in_console=false\n",
                    ]
                )

            ctx = self.cl.create_some_context(0)
            dev = ctx.devices[0]
            cmds = self.cl.CommandQueue(ctx, dev)
            prg = self.cl.Program(ctx, [dev], [xclbin_source])

            prg.build()

            # Work around an intermittent PyOpenCL bug. Using prg.Toplevel
            # internally accesses prg._source, expecting it to be a normal
            # attribute instead of a kernel name.
            kern = self.cl.Kernel(prg, "Toplevel")

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
            kern(cmds, (1,), (1,), np.uint32(10000), *buffers.values())
            end_time = time.time()
            log.debug(f"Emulation time: {end_time - start_time} sec")

            # read the result
            output = {"memories": {}}
            for name, buf in buffers.items():
                out_buf = np.zeros_like(data[name]["data"]).astype(np.uint32)
                self.cl.enqueue_copy(cmds, out_buf, buf)
                buf.release()
                output["memories"][name] = list(map(lambda x: int(x), out_buf))

            # cleanup
            del ctx

            # Add xrt log output to our debug output.
            if os.path.exists(xrt_output_logname):
                log.debug("XRT log:")
                with open(xrt_output_logname, "r") as f:
                    for line in f.readlines():
                        log.debug(line.strip())

            # And, in emulation mode, also include the emulation log.
            emu_log = "emulation_debug.log"
            if os.path.exists(emu_log):
                log.debug("Emulation log:")
                with open(emu_log, "r") as f:
                    for line in f.readlines():
                        log.debug(line.strip())

            return sjson.dumps(output, indent=2, use_decimal=True)

        import_libs()
        res = run(input_data)
        return res
