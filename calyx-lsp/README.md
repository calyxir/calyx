# Language Server for Calyx

## Installing

Build the repo with `cargo build`. This uses `build.rs` to build and link the tree-sitter dependencies. Then, link the resulting binary into a place on your path. I like `~/.local/bin`.

```bash
cd ~/.local/bin
ln -s $calyx_repo/target/debug/calyx-lsp calyx-lsp
```


### Note from Ethan:

I made a hacky install script if you want to use it:
```shell
yes | ./install.sh
```

You can, of course, customize what it does by omitting `yes | ` and answering "No" to some of the defaults.
(Remember to `sudo rm` the original location first.)
