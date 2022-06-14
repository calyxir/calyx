import logging as log
import os
import time

import numpy as np
import simplejson as sjson

from fud import errors
from fud.stages import SourceType, Stage
from fud.utils import FreshDir, TmpDir
from shutil import rmtree


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

    def _define_steps(self, input, builder, config):
        data_path = config["stages", self.name, "data"]
        
        # Has same problem as emulation.py: any input of form
        # '-s fpga.save_temps <any non-empty string>' is True
        save_wdb = bool(config["stages", self.name, "waveform"]) and config["stages", self.name, "waveform"] == "true"
        
        # TODO: delete print("\ncwd is: " + os.getcwd())    
        @builder.step()
        def import_libs():
            """Import optional libraries"""
            try:
                import pyopencl as cl  # type: ignore

                self.cl = cl
            except ImportError:
                raise errors.RemoteLibsNotInstalled

        @builder.step()
        def run(xclbin: SourceType.Path) -> SourceType.String:
            """Run the xclbin with datafile"""

            if data_path is None:
                raise errors.MissingDynamicConfiguration("fpga.data")

            #TODO: delete print("cwd2 is: " + os.getcwd())    
            data = sjson.load(open(data_path), use_decimal=True)
            xclbin_source = xclbin.open("rb").read()

            # create a temporary directory with an xrt.ini file that redirects
            # the runtime log to a file so that we can control how it's printed.
            # This is hacky, but it's the only way to do it
            # As a result the xrt.init in fud/bitstream is ignored

            new_dir = FreshDir() if save_wdb else TmpDir()
            os.chdir(new_dir.name)

            xrt_output_logname = "output.log"
            with open("xrt.ini", "w") as f:
               xrt_ini_config = [
                                    "[Runtime]\n",
                                    f"runtime_log={xrt_output_logname}\n",
                                    "[Emulation]\n",
                                    "print_infos_in_console=false\n"
                                ]
               if(save_wdb):
                   xrt_ini_config.append("debug_mode=batch\n")

               f.writelines(xrt_ini_config)

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
                    # TODO: use real type information
                    hostbuf=np.array(data[mem]["data"]).astype(np.uint32),
                )
                # TODO: use real type information
                buffers[mem] = buf

            start_time = time.time()
            #Note that this is the call on v++. This uses global USER_ENV variables
            #EMCONFIG_PATH=`pwd`
            #XCL_EMULATION_MODE=hw_emu
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
        res = run(input)
        return res

#        # cleanup fud_out if flagged
#        # TODO: fix this, at the moment runs too soon and is not capable
#        # of cleaning anything up
#        if(save_wdb):
#            
#            print(self._latest_dir(os.getcwd()))
#            entries = os.scandir(self._latest_dir(os.getcwd()))
#            for entry in entries:
#                print(entry.name)
#                # if(not ".wdb" in entry.path or not ".wcfg" in entry.path):
#                #     if entry.is_dir():
#                #         rmtree(entry.path)
#                #     else:
#                #         print("here")
#                #         os.remove(entry.path)
#        return res
#
#
#    # returns path of most recent directory created by
#    # FreshDir() (see fud/fud/utils.py)
#    def _latest_dir(self, cwd):
#        
#        i = 0
#        file_convention = "fud-out-{}"
#        name = file_convention.format(i + 1)
#        path = os.path.join(cwd, name)
#        while os.path.exists(path):
#            i += 1
#            name = file_convention.format(i + 1)
#            path = os.path.join(cwd,name)
#        name = file_convention.format(i)
#        return os.path.join(cwd, name)    

