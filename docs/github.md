# Contributing to Calyx

## A few notes on Github practices around the Calyx project
The current home of the Calyx repo can be found [here][calyx_repo]. As with many
large projects, we protect the main branch of the repo so that updates can only
be made via pull requests. So the development cycle tends to look like:
```
checkout main -> develop code -> open PR -> revise -> merge PR
```

For legibility of commits, we squash all commits in a PR down to a single commit
and merge the composite commit to the main branch. This helps keep the commit
count of the main branch lower than it would otherwise be, however it can make
using commands like `git bisect` more challenging for large branches. For that
reason we tend to recommend more frequent PRs to avoid large deltas.

Once your PR has been merged, be sure to ***checkout from the updated main branch***
for future changes. If you branch off the merged branch or continue with it,
there will be extensive merge conflicts due to the squash and merge tactic. For
this reason we always recommend creating branches off of the main branch if you
intend to have them merged into it.


[calyx_repo]: https://github.com/calyxir/calyx
