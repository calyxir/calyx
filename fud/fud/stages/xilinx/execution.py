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

        # Make a temporary sandbox directory for the execution.
        new_dir = FreshDir() if save_temps else TmpDir()
        xrt_ini_path = os.path.join(new_dir.name, "xrt.ini")

        @builder.step()
        def configure():
            """Create config files based on fud arguments"""

            self.xrt_output_logname = "output.log"
            with open(xrt_ini_path, "w") as f:
                xrt_ini_config = [
                    "[Runtime]\n",
                    f"runtime_log={self.xrt_output_logname}\n",
                    "[Emulation]\n",
                    "print_infos_in_console=true\n",
                ]
                if waveform:
                    pre_sim_path = os.path.join(new_dir.name, "pre_sim.tcl")
                    post_sim_path = os.path.join(new_dir.name, "post_sim.tcl")
                    xrt_ini_config.append("debug_mode=batch\n")
                    xrt_ini_config.append(f"user_pre_sim_script={pre_sim_path}\n")
                    xrt_ini_config.append(f"user_post_sim_script={post_sim_path}\n")
                f.writelines(xrt_ini_config)

            # Extra Tcl scripts to produce a VCD waveform dump.
            if waveform:
                with open(pre_sim_path, "w") as f:
                    f.writelines(
                        [
                            "open_vcd\n",
                            "log_vcd *\n",
                        ]
                    )
                with open(post_sim_path, "w") as f:
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
        def run(xclbin: SourceType.Path) -> SourceType.String:
            """Run the xclbin with datafile"""

            if data_path is None:
                raise errors.MissingDynamicConfiguration("fpga.data")

            # Build the xclrun command line.
            data_abs = Path(data_path).resolve()
            xclbin_abs = xclbin.resolve()
            out_json = Path(new_dir.name).joinpath("out.json")
            shell_cmd = (
                f"source {vitis_path}/settings64.sh ; "
                f"source {xrt_path}/setup.sh ; "
                f"{sys.executable} -m fud.stages.xilinx.xclrun "
                f"--out {out_json} "
                f"{xclbin_abs} {data_abs}"
            )
            envs = {
                "EMCONFIG_PATH": new_dir.name,  # XXX(samps): Generate this with emconfigutil!
                "XCL_EMULATION_MODE": emu_mode,  # hw_emu or hw
                "XRT_INI_PATH": xrt_ini_path,
            }

            # Invoke xclrun.
            start_time = time.time()
            shell(
                ["bash", "-c", shlex.quote(shell_cmd)],
                env=envs,
                cwd=new_dir.name,
                capture_stdout=False,
                stdout_as_debug=True,
            )
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

            # It would be nice if we could return this as a file without
            # reading it, but it's in a temporary directory that's about to be
            # deleted.
            with open(out_json) as f:
                return f.read()

        configure()
        res = run(input)
        return res
