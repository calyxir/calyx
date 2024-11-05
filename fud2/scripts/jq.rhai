import "rtl_sim" as sim;

export const jq_state = state("jq", ["jq"]);

defop dat_to_jq(json: sim::dat) >> out: jq_state {
    let expr = config_or("jq.expr", ".");
    let jq = config_or("jq.exe", "jq");
    let flags = config_or("jq.flags", "");

    shell(`${jq} '${expr}' ${flags} ${json} > ${out}`);
}