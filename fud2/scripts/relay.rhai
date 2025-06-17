import "calyx" as calyx;

export const relay_state = state("relay", ["relay"]);

defop relay(input: relay_state) >> out: calyx::calyx_state {
    let python = config_or("python", "python3");
    let calyx_base = config_or("calyx.base", "");
    let relay_exe = config_or("relay-exe", `${calyx_base}/frontends/relay/relay_visitor.py`);

    shell(`${python} ${relay_exe} ${input} > ${out}`)
}
