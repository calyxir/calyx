#lang racket/base

(require racket/cmdline)

(define visual-mode (make-parameter 'none))
(define data-filename (make-parameter #f))

(command-line
 #:program "test"
 #:once-any
 [("-s" "--structure") "Show just the structure of the circuit"
                       (visual-mode 'structure)]
 [("-a" "--animate") data-path
                     "Animate the circuit with <data-path> as input."
                     (visual-mode 'animate)
                     (data-filename data-path)])

(displayln (visual-mode))
(displayln (data-filename))
