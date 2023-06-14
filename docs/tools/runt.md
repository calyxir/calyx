# Runt

Runt (Run Tests) is the expectation testing framework for Calyx. It organizes
collections of tests into test suites and specifies configuration for them.

Runt uses `runt.toml` to define the test suites and configure them.

## Cheatsheet

Runt workflow involves two things:
1. Running tests and comparing differences
2. Saving new or changed golden files

To run all the tests in a directory, run `runt` with a folder containing `runt.toml`.

The following commands help focus on specific tests to run:
- `-i`: Include files that match the given pattern. The pattern is matched against `<suite name>:<file path>` so it can be used to filter both test suites or specific paths. General regex patterns are supported.
- `-x`: Exclude files that match the pattern
- `-o`: Filter out reported test results based on test status. Running with `miss` will only show the tests that don't have an `.expect` file.

**Differences**. `-d` or `--diff` shows differences between the expected test output and the generated output. Use this in conjunction with `-i` to focus on particular failing tests.

**Saving Files**. `-s` is used to save test outputs when they have expected changes. In the case of `miss` tests, i.e. tests that currently don't have any expected output file, this saves a completely new `.expect` file.

**Dry run**. `-n` flag shows the commands that `runt` will run for each test. Use this when you directly want to run the command for the test directly.

For other options, look at `runt --help` which documents other features in `runt`.

For instruction on using runt, see the [official documentation](https://docs.rs/runt/latest/runt/).
