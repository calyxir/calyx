const s1 = state("s1", []);
const s2 = state("s2", []);
const s3 = state("s3", []);
const s4 = state("s4", []);
const s5 = state("s5", []);
const s6 = state("s6", []);

// Testing basic multiple states in multiple out
op_multi("t1", [], [s1, s2], [s3, s4], |e, input, output| {e.build_cmd(input, "test", output, [])})
