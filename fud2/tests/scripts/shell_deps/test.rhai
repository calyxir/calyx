const s1 = state("s1", []);
const s2 = state("s2", []);
const s3 = state("s3", []);
const s4 = state("s4", []);
const s5 = state("s5", []);
const s6 = state("s6", []);

// Testing all outputs covered by `shell_deps` and general enumerations.
defop t1(a: s1) >> b: s2 {
  shell_deps("echo one", [], ["f1"]);
  shell_deps("echo two", ["f1"], []);
  shell_deps("echo four", [], []);
  shell_deps("echo five", ["f1"], ["f2"]);
  shell_deps("echo five", [a], []);
  shell_deps("echo five", [], [b]);
  shell_deps("echo five", [a, "f1", "f2"], ["f4", "f5"]);
}

// Testing not having all outputs covered by `shell_deps` commands.
defop t2(a: s3) >> b: s4 {
  shell_deps("echo five", [a], ["f4", "f5"]);
}

// Testing `shell`.
defop t3(a: s5) >> b: s6 {
  shell("echo Char");
  shell("echo Ray");
  shell("echo Saura");
}
