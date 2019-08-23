#lang racket/base

(require racket/cmdline
         racket/match
         "ast.rkt"
         "json.rkt"
         "visualizer.rkt")

(provide parse-cmdline)

(define (parse-cmdline comp [filename #f] [visualizer #t])

  (define mode (make-parameter 'none))
  (define data-filename (make-parameter #f))

  (if filename
      (if visualizer
          (plot-compute comp '() #:memory (json->memory filename))
          (void (compute comp '()
                         #:memory (json->memory filename)
                         #:toplevel #t)))
      (begin
        (command-line
         #:program "test"
         #:once-any
         [("-s" "--structure") "Show just the structure of the circuit."
                               (mode 'structure)]
         [("-a" "--animate") data-path
                             "Animate the circuit with <data-path> as input."
                             (mode 'animate)
                             (data-filename data-path)]
         [("-c" "--compute") data-path
                             "Output result of computing circuit with <data-path> as input."
                             (mode 'compute)
                             (data-filename data-path)]
         )

        (match (mode)
          ['compute (void (compute comp '()
                                   #:toplevel #t
                                   #:memory (json->memory (data-filename))))]
          ['structure (plot-component comp)]
          ['animate (plot-compute comp '() #:memory (json->memory (data-filename)))]
          ['none (displayln "You have to provide an option! Use the -h flag for more information.")
                 (exit -1)]))))

