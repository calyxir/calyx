#lang racket/base
(require "futil-syntax.rkt"
         "futil-prims.rkt"
         "visualizer.rkt")

(plot-compute
 (comp/iterator)
 '((start . 0)
   (incr . 2)
   (end . 10)
   (en . 1)))

(define/module client () ((out : 32) (stop : 32))
  ([iter = new comp/iterator]
   [const start 0 : 32 -> iter @ start]
   [const incr 2 : 32 -> iter @ incr]
   [const end 10 : 32 -> iter @ end]
   [const en 1 : 32 -> iter @ en]

   [iter @ out -> out]
   [iter @ stop -> stop])
  []
  [(!! en iter)]
  [(!! en iter)]
  [(!! en iter)]
  [(!! en iter)]
  [(!! en iter)]
  [(!! en iter)])

(plot-component (comp/iterator))

(displayln "XXX here 1")
(show-debug
 (plot-compute (client) '()))


(display-mem
 'C
 (ast-tuple '() '() '()
            (json->memory "../benchmarks/linear-algebra-2mm.json")))
