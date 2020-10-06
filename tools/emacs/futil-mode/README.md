# Fuse Mode
This provides simple syntax highlighting and indentation for Fuse in Emacs.

## Installation
Clone this repository to a location of your choice. Add it to the laod path, and then require `fuse-mode`. 
For Spacemacs, this looks like adding the following lines to `dotspacemacs/user-config` in your `.spacemacs` file:
```elisp
(push "~/.emacs.d/private/local/fuse-mode" load-path)
(require 'fuse-mode)
```
I imagine it looks very similar for pure emacs, but haven't actually tried it myself.

## Known Bugs
 - The indentation code isn't aware of comments which means that a lone bracket in a comment will throw off indentation.
 - Negative numbers not highlighted
