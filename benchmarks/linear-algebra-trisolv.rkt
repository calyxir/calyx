#lang racket
(require "ast.rkt"
         "futil.rkt"
         "futil-prims.rkt"
         "dis-graphs.rkt")

;; I don't think that this module is possible to define at the moment
;; because a submodule has no way of changing the state of the parent module
;; XXXX XXXX XXXX XXXX
;; (define/module j_lt_i ((j : 32) (i : 32)) )

;; registers are local variables
;; I think that I need global variables

(define/module loop (()) (())
  ([])
  )
