const state0 = state("state0", []);
const state1 = state("state1", []);
const state2 = state("state2", []);
const state3 = state("state3", []);
const state4 = state("state4", []);

// simple defop
defop t0(s0: state0) >> s1: state1 {
  shell("echo uwu");
}

// defop combining states
defop t1(s0: state0, s1: state1) >> s2: state2 {
  shell("echo owo");
}

// defop splitting states
defop t2(s0: state0) >> s1: state1, s2: state2 {
  shell("echo -_-");
}

// testing input and output variables
defop t3(s0: state0, s1: state1, s2: state2) >> s3: state3, s4: state4 {
  shell(`echo inputs ${s0} ${s1} ${s2}`);
  shell(`echo outputs ${s3} ${s4}`);
}

// testing config works
defop t4(s0: state0) >> s1: state1 {
  let c0 = config("c0");
  shell(`echo ${c0}`);
}

// testing config_or works
defop t5(s0: state0) >> s1: state1 {
  let c0 = config_or("this-config-better-not-exist", "gholdengo");
  shell(`echo ${c0}`);
}
