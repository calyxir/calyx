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

        # Configuration for the xclrun command.
        vitis_path = config["stages", self.name, "xilinx_location"]
        xrt_path = config["stages", self.name, "xrt_location"]
        emu_mode = config["stages", "xclbin", "mode"]

        # Make a temporary sandbox directory for the execution.
        new_dir = FreshDir() if save_temps else TmpDir()
        xrt_ini_path = os.path.join(new_dir.name, "xrt.ini")

        @builder.step()
        def configure():
            """Create config files based on fud arguments"""

            # Create the main `xrt.ini` file that configures simulation.
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

            # In waveform mode, add a couple of Tcl scripts that are necessary
            # to dump a VCD.
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

            # Create the `emconfig.json` file that the simulator loudly (but
            # perhaps unnecessarily?) complains about if it's missing.
            if emu_mode != 'hw':
                platform = config["stages", "xclbin", "device"]
                utilpath = os.path.join(vitis_path, 'bin', 'emconfigutil')
                shell(
                    f'{utilpath} --platform {platform} --od {new_dir.name}',
                    capture_stdout=False,
                    stdout_as_debug=True,
                )

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
                f"{sys.executable} -m fud.xclrun "
                f"--out {out_json} "
                f"{xclbin_abs} {data_abs}"
            )
            envs = {
                "XRT_INI_PATH": xrt_ini_path,
            }
            if emu_mode != 'hw':
                # `hw` denotes actual hardware execution. In other modes,
                # configure emulation.
                envs.update({
                    "EMCONFIG_PATH": new_dir.name,
                    "XCL_EMULATION_MODE": emu_mode,  # hw_emu or hw
                })

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
            log.debug(f"Execution time: {end_time - start_time} sec")

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
