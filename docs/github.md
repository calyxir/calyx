# Contributing to Calyx

## Github Workflow
The current home of the Calyx repo can be found [here][calyx_repo]. As with many
large projects, we protect the main branch of the repo so that updates can only
be made via pull requests. So the development cycle tends to look like:
```
checkout main -> develop code -> open PR -> revise -> merge PR
```

For legibility of commits, we squash all commits in a PR down to a single commit
and merge the composite commit to the main branch. This helps keep the commit
count of the main branch lower than it would otherwise be; however, it can make
using commands like `git bisect` more challenging for large branches. For that
reason we tend to recommend more frequent PRs to avoid large deltas.

Once your PR has been merged, be sure to ***check out the updated main branch***
for future changes. If you branch off the merged branch or continue with it,
there will be extensive merge conflicts due to the squash and merge tactic. For
this reason we always recommend creating branches off of the main branch if you
intend to have them merged into it.

### CI Behavior
The CI runs a number of tests including ensuring that Rust and Python code has
been formatted. For Python we use the [Black](https://github.com/psf/black) formatter and for Rust we use the
standard `cargo fmt`.

For Rust further linting is done via [`clippy`][clippy] to ensure that there are
no warnings. In situations where warnings are expected, such as code that is
only part way through development, you can opt to add `#[allow]` annotations
within Rust to suppress the lint.

If changes are made to the `Dockerfile` then the CI will automatically rebuild
the Docker image and run your tests on it.


[calyx_repo]: https://github.com/calyxir/calyx
[clippy]: https://github.com/rust-lang/rust-clippy
