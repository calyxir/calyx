import logging as log
import os
import time

import simplejson as sjson


from fud import errors
from fud.stages import SourceType, Stage
from fud.utils import FreshDir, TmpDir
from pathlib import Path


class HwExecutionStage(Stage):
    name = "fpga"

    def __init__(self):
        super().__init__(
            src_state="xclbin",
            target_state="fpga",
            input_type=SourceType.Path,
            output_type=SourceType.String,
            description="Run an xclbin on an fpga, or emulate hardware execution",
        )

    def _define_steps(self, input, builder, config):
        data_path = config["stages", self.name, "data"]

        save_temps = bool(config["stages", self.name, "save_temps"])
        waveform = bool(config["stages", self.name, "waveform"])
        if waveform and not save_temps:
            log.warn(
                f"{self.name}.waveform is enabled, but {self.name}.save_temps "
                f"is not. This will generate a WDB file but then immediately "
                f"delete it. Consider adding `-s {self.name}.save_temps true`."
            )

        @builder.step()
        def configure():
            """Create config files based on fud arguments"""

            os.chdir(new_dir.name)

            self.xrt_output_logname = "output.log"
            with open("xrt.ini", "w") as f:
                xrt_ini_config = [
                    "[Runtime]\n",
                    f"runtime_log={self.xrt_output_logname}\n",
                    "[Emulation]\n",
                    "print_infos_in_console=false\n",
                ]
                if waveform:
                    xrt_ini_config.append("debug_mode=batch\n")
                    xrt_ini_config.append(
                        f"user_pre_sim_script={new_dir.name}/pre_sim.tcl\n"
                    )
                    xrt_ini_config.append(
                        f"user_post_sim_script={new_dir.name}/post_sim.tcl\n"
                    )
                f.writelines(xrt_ini_config)

            # Extra Tcl scripts to produce a VCD waveform dump.
            if waveform:
                with open("pre_sim.tcl", "w") as f:
                    f.writelines(
                        [
                            "open_vcd\n",
                            "log_vcd *\n",
                        ]
                    )
                with open("post_sim.tcl", "w") as f:
                    f.writelines(
                        [
                            "close_vcd\n",
                        ]
                    )
            # NOTE(nathanielnrn): It seems like this must come after writing
            # the xrt.ini file in order to work
            os.environ["XRT_INI_PATH"] = f"{new_dir.name}/xrt.ini"

        @builder.step()
        def import_libs():
            """Import optional libraries"""
            try:
                from fud.stages.xilinx import fud_pynq_script

                self.pynq_script = fud_pynq_script
            except ImportError:
                raise errors.RemoteLibsNotInstalled

        @builder.step()
        def run(xclbin: SourceType.Path) -> SourceType.String:
            """Run the xclbin with datafile"""

            if data_path is None:
                raise errors.MissingDynamicConfiguration("fpga.data")
            # Solves relative path messiness
            orig_dir_path = Path(orig_dir)
            abs_data_path = orig_dir_path.joinpath(Path(data_path)).resolve()
            abs_xclbin_path = orig_dir_path.joinpath(xclbin).resolve()

            data = sjson.load(open(abs_data_path), use_decimal=True)
            start_time = time.time()
            # Note that this is the call on v++. This uses global USER_ENV variables
            # EMCONFIG_PATH=`pwd`
            # XCL_EMULATION_MODE=hw_emu
            kernel_output = self.pynq_script.run(abs_xclbin_path, data)
            end_time = time.time()
            log.debug(f"Emulation time: {end_time - start_time} sec")

            # Add xrt log output to our debug output.
            if os.path.exists(self.xrt_output_logname):
                log.debug("XRT log:")
                with open(self.xrt_output_logname, "r") as f:
                    for line in f.readlines():
                        log.debug(line.strip())

            # And, in emulation mode, also include the emulation log.
            emu_log = "emulation_debug.log"
            if os.path.exists(emu_log):
                log.debug("Emulation log:")
                with open(emu_log, "r") as f:
                    for line in f.readlines():
                        log.debug(line.strip())

            return sjson.dumps(kernel_output, indent=2, use_decimal=True)

        orig_dir = os.getcwd()
        # Create a temporary directory (used in configure()) with an xrt.ini
        # file that redirects the runtime log to a file so that we can control
        # how it's printed. This is hacky, but it's the only way to do it.
        # (The `xrt.ini`file we currently have in `fud/bitstream` is not used here.)
        new_dir = FreshDir() if save_temps else TmpDir()

        configure()
        import_libs()
        res = run(input)
        return res
