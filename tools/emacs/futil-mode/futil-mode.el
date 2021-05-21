;;; packages.el --- custom layer packages file for Spacemacs.
;;
;; Copyright (c) 2012-2016 Sylvain Benner & Contributors
;;
;; Author: Sam Thomas <samthomas@samthomas>
;; URL: https://github.com/syl20bnr/spacemacs
;;
;; This file is not part of GNU Emacs.
;;
;;; License: GPLv3

;;; Commentary:

;; See the Spacemacs documentation and FAQs for instructions on how to implement
;; a new layer:

;;
;;   SPC h SPC layers RET
;;
;;
;; Briefly, each package to be installed or configured by this layer should be
;; added to `custom-packages'. Then, for each package PACKAGE:
;;
;; - If PACKAGE is not referenced by any other Spacemacs layer, define a
;;   function `custom/init-PACKAGE' to load and initialize the package.

;; - Otherwise, PACKAGE is already referenced by another Spacemacs layer, so
;;   define the functions `custom/pre-init-PACKAGE' and/or
;;   `custom/post-init-PACKAGE' to customize the package as it is loaded.

;;; Code:

(require 'highlight-numbers)

(defvar futil-mode-map
  (let ((map (make-sparse-keymap)))
    (define-key map "\C-j" 'newline-and-indent)
    map)
  "Keymap for `futil-mode'.")

(defvar futil-mode-syntax-table
  (let ((st (make-syntax-table)))
    ;; comments
    (modify-syntax-entry ?/ ". 12b" st)
    (modify-syntax-entry ?* ". 23b" st)
    (modify-syntax-entry ?\n "> b" st)

    ;; strings
    (modify-syntax-entry ?\" "\"" st)
    st)
  "Syntax table for `futil-mode'.")

(setq futil-font-lock-keywords
  (let* ((futil-defn '("component" "cells" "wires" "control" "primitive"))
         (futil-control '("seq" "par" "if" "while" "else" "with" "invoke"))
         (futil-keywords '("prim" "import" "group"))

         (futil-defn-regexp (regexp-opt futil-defn 'words))
         (futil-control-regexp (regexp-opt futil-control 'words))
         (futil-keywords-regexp (regexp-opt futil-keywords 'word))
         (futil-attributes-regexp "\\(@\\(?:[[:alpha:]]\\|_\\)+\\)\\(?:(.*?)\\)"))

    `((,futil-defn-regexp . (1 font-lock-keyword-face))
      (,futil-control-regexp . (1 font-lock-type-face))
      (,futil-keywords-regexp . (1 font-lock-constant-face))
      (,futil-attributes-regexp . (1 font-lock-type-face)))))

 ;;; Indentation

(defvar futil-indent-level 2)

(defun futil-count-back ()
  (let ((count 0)
        (not-top t))
    (save-excursion
      (end-of-line)
      (forward-char -1)
      (if (looking-at "{")
          (forward-char -1))
      (while not-top
        (if (looking-at "}")
            (setq count (- count 1)))
        (if (looking-at "{")
            (setq count (+ count 1)))
        (forward-char -1)
        (if (bobp)
            (setq not-top nil)))
      count)))

(defun futil-print-back ()
  (interactive)
  (message "Back: %s" (futil-count-back)))

(defun futil-indent-line ()
  (interactive)
  (end-of-line)
  (indent-line-to (* futil-indent-level (futil-count-back))))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.futil\\'" . futil-mode))

(define-derived-mode futil-mode prog-mode "Futil Mode"
  "A major mode for editing Futil source files."
  :syntax-table futil-mode-syntax-table
  (setq-local comment-start "// ")
  (setq-local comment-end "")
  (setq-local comment-start-skip "//+\\s-*")
  (setq-local font-lock-defaults
              '((futil-font-lock-keywords)))
  (setq-local indent-line-function 'futil-indent-line)
  (puthash 'futil-mode
           "\\(\\<[[:digit:]]+\\(?:'[bdxo]\\([[:digit:]]\\|[A-Fa-f]\\)+\\)?\\>\\)"
           highlight-numbers-modelist))

(provide 'futil-mode)

;;; packages.el ends here
