import "rtl_sim" as sim;

defop vcd_json(vcd: sim::vcd) >> out: sim::dat {
    let vcdump = config_or("vcdump.exe", "vcdump");

    shell(`${vcdump} ${vcd} > ${out}`)
}
