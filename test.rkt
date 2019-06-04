#lang racket
(require "component.rkt"
         "futil.rkt")
         "futil-prims.rkt"

(define/module myadd ((lft : 32) (rgt : 32)) ((out : 32))
  [adder = new comp/add]
  [lft -> adder @ left]
  [rgt -> adder @ right]
  [adder @ out -> out])
;; (plot (myadd))

(define/module splitter32 ((in : 32)) ((out-l : 20) (out-r : 12))
  [in-l & in-r = split 20 in]
  [in-l -> out-l]
  [in-r -> out-r])
;; (plot (splitter32))

(define/module joiner32 ((in-l : 16) (in-r : 16)) ((out : 32))
  [out-l & out-r = split 16 out]
  [in-l -> out-l]
  [in-r -> out-r])
;; (plot (joiner32))
