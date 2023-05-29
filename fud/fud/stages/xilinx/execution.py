import logging as log
import os
import time
import sys
import shlex

from fud import errors
from fud.stages import SourceType, Stage
from fud.utils import FreshDir, TmpDir, shell
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
                    "print_infos_in_console=true\n",
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

        # Configuration for the xclrun command.
        vitis_path = config["stages", self.name, "xilinx_location"]
        xrt_path = config["stages", self.name, "xrt_location"]
        emu_mode = config["stages", "xclbin", "mode"]

        @builder.step()
        def run(xclbin: SourceType.Path) -> SourceType.Stream:
            """Run the xclbin with datafile"""

            if data_path is None:
                raise errors.MissingDynamicConfiguration("fpga.data")

            # Resolve paths relative to original directory.
            # XXX(samps): This should not be necessary if we don't change dirs.
            data_abs = Path(orig_dir).joinpath(Path(data_path)).resolve()
            xclbin_abs = Path(orig_dir).joinpath(xclbin).resolve()

            # Build the xclrun command line.
            shell_cmd = (
                f"source {vitis_path}/settings64.sh ; "
                f"source {xrt_path}/setup.sh ; "
                f"{sys.executable} -m fud.stages.xilinx.xclrun {xclbin_abs} {data_abs}"
            )
            envs = {
                "EMCONFIG_PATH": orig_dir,  # XXX(samps): Generate this with emconfigutil!
                "XCL_EMULATION_MODE": emu_mode,  # hw_emu or hw
                "XRT_INI_PATH": f"{new_dir.name}/xrt.ini",
            }

            # Invoke xclrun.
            start_time = time.time()
            kernel_output = shell(["bash", "-c", shlex.quote(shell_cmd)], env=envs)
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

            return kernel_output

        orig_dir = os.getcwd()
        # Create a temporary directory (used in configure()) with an xrt.ini
        # file that redirects the runtime log to a file so that we can control
        # how it's printed. This is hacky, but it's the only way to do it.
        # (The `xrt.ini`file we currently have in `fud/bitstream` is not used here.)
        new_dir = FreshDir() if save_temps else TmpDir()

        configure()
        res = run(input)
        return res
