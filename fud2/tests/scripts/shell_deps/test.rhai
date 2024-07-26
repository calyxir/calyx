const s1 = state("s1", []);
const s2 = state("s2", []);

defop t1(a: s1) >> b: s2 {
  shell("echo uwu");
  shell_deps("echo one", [], ["f1"]);
  shell_deps("echo two", ["f1"], []);
  shell("echo three");
  shell_deps("echo four", [], []);
  shell_deps("echo five", ["f1"], ["f2"]);
  shell_deps("echo five", [a], []);
  shell_deps("echo five", [], [b]);
  shell_deps("echo five", [a, "f1", "f2"], ["f4", "f5", b]);
}
