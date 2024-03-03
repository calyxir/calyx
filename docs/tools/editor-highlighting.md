# Editor Highlighting

## Language Server

There is a Calyx language server that provides jump-to-definition and completion support. Instructions for intsalling it are [here](./language-server.md).

If you are using any of the `unsyn-*` primitives, you will need to tell the language server to use the Calyx repo as the library location instead of the default `~/.calyx`. Below, there are instructions on how to do this for each editor.

## Vim

The vim extension highlights files with the extension `.futil`.
It can be installed using a plugin manager such as [vim-plug][] using a
local installation.
Add the following to your vim plug configuration:

```
Plug '<path-to-calyx>/tools/vim'
```

And run:

```
:PlugInstall
```

If you use lazy.nvim, you can add the following block:

```lua
{
  dir = "<path-to-calyx>/tools/vim/futil",
  config = function()
    require("futil").setup({
      -- optionally specify a custom library location
      calyxLsp = {
        libraryPaths = {
          "<path-to-calyx>"
        }
      }
    })
  end
}
```

## Emacs

`calyx-mode` is implements tree-sitter based highlighting for `.futil` files in emacs. It's located [here](calyx-mode).

You can install it with `straight.el` or `elpaca` like so:

```lisp
(use-package calyx-mode
  :<elpaca|straight> (calyx-mode :host github :repo "sgpthomas/calyx-mode")
  :config
  (setq-default eglot-workspace-configuration
                '(:calyx-lsp (:library-paths ["<path-to-calyx>"]))))
```

## Visual Studio Code

You can install the Calyx extension from the extension store. To specify a custom library location, go to the Calyx extension settings, and edit the `calyxLsp.libraryPaths` key to point to the root Calyx repository.

[vim-plug]: https://github.com/junegunn/vim-plug
[spacemacs]: https://www.spacemacs.org/
[load path]: http://www.emacswiki.org/emacs/LoadPath
[calyx-mode]: https://github.com/sgpthomas/calyx-mode
