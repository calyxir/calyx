const fs = require("fs");
const path = require("path");
const childProcess = require("child_process");
const config = require("./data/config.json");
const passes = require("./data/passes.json").passes.map((p) => p.name);
const examples = config.examples;

const PASS_OUTPUT_PREFIX = "- all:";

// we assume that we have a debug build of calyx ready
const _spawned = childProcess.spawnSync(
  path.join(__dirname, "..", "target/debug/calyx"),
  ["pass-help"],
);
const op = new String(_spawned.stdout);
const compileAliases = op.split("Aliases:\n")[1];
const allPasses = compileAliases
  .split("\n")
  .filter((s) => s.startsWith(PASS_OUTPUT_PREFIX))[0]
  .replace(PASS_OUTPUT_PREFIX, "");

const errors = [];

for (const pass of passes) {
  if (!allPasses.includes(pass)) {
    errors.push(`pass ${pass} is not valid`);
  }
}

const validPasses = passes.filter((p) => allPasses.includes(p));

for (const eg of examples) {
  const filepath = path.join(__dirname, "..", eg.file);
  if (!fs.existsSync(filepath)) {
    errors.push(`file ${eg.file} does not exist`);
  }
  for (const pass of eg.passes) {
    if (!validPasses.includes(pass)) {
      errors.push(`pass ${pass} for example ${eg.name} is invalid`);
    }
  }
}
if (errors.length > 0) {
  console.error(errors);
  process.exit(1);
}
