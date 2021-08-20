# Editor Highlighting

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

## Emacs

`futil-mode` is implements highlighting for `.futil` files in emacs.
It is located in `<repo>/tools/emacs/futil-mode`.

Clone the repository, add the above path to your [load path][], and require
`futil-mode`.
If you use [Spacemacs][], this looks like adding the following lines to
`dotspacemacs/user-config` in your `.spacemacs` file:
```elisp
(push "~/.emacs.d/private/local/fuse-mode" load-path)
(require 'fuse-mode)
```
I imagine it looks very similar for pure emacs, but haven't actually tried it myself.

## Visual Studio Code

Add a link to the Calyx VSCode extension directory to your VSCode extensions directory.
```
cd $HOME/.vscode/extensions
ln -s <calyx root directory/tools/vscode calyx.calyx-0.0.1
```
Restart VSCode.

[vim-plug]: https://github.com/junegunn/vim-plug
[spacemacs]: https://www.spacemacs.org/
[load path]: http://www.emacswiki.org/emacs/LoadPath
