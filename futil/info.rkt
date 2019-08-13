#lang info
(define collection "futil")
(define deps '("base"))
(define build-deps '("scribble-lib"
                     "racket-doc"
                     "rackunit-lib"
                     "graph"
                     "threading-lib"))
;; (define scribblings '(("scribblings/futil.scrbl" ())))
(define pkg-desc "Description Here")
(define version "1.0.0")
(define pkg-authors '(Samuel Thomas))
