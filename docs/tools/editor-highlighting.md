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

The `highlight-numbers` package is required as part of `futil-mode`, install it:
```
M-x package-install RET highlight-numbers RET
```

Clone the repository, add the above path to your [load path][], and require
`futil-mode` in your `.emacs` file:
```elisp
(push "~/.emacs.d/private/local/futil-mode" load-path)
(require 'futil-mode)
```

If you use [Spacemacs][], you would add this to `dotspacemacs/user-config`
in your `.spacemacs`.


## Visual Studio Code

Add a link to the Calyx VSCode extension directory to your VSCode extensions directory.
```
cd $HOME/.vscode/extensions
ln -s <calyx root directory>/tools/vscode calyx.calyx-0.0.1
```
Restart VSCode.

[vim-plug]: https://github.com/junegunn/vim-plug
[spacemacs]: https://www.spacemacs.org/
[load path]: http://www.emacswiki.org/emacs/LoadPath
