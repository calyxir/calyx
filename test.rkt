#lang racket
(require "component.rkt"
         "fuse-ir.rkt")
         ;; "fuse-ir-prims.rkt"

;; (define/module myadd (l r) (o)
;;   ([adder = new comp/add]
;;    [l -> adder @ left]
;;    [r -> adder @ right]
;;    [adder @ out -> o]))
;; (myadd)
;; (plot (myadd))

(define (comp/add)
  (default-component 'add
    (list (port 'left 32)
          (port 'right 32))
    (list (port 'out 32))))

(define/module myadd ((lft : 32) (rgt : 32)) ((out : 32))
  [adder = new comp/add]
  [lft -> adder @ left]
  [rgt -> adder @ right]
  [adder @ out -> out])
;; (plot (myadd))

(define (myadd-man)
  (define c (default-component 'myadd-man (list (port 'l 32) (port 'r 32)) (list (port 'o 32))))
  (add-submod! c 'adder (comp/add))
  (connect! c 'l 'inf# 'adder 'left)
  (connect! c 'r 'inf# 'adder 'right)
  (connect! c 'adder 'out 'o 'inf#)
  c)
(myadd-man)

(define/module splitter32 ((in : 32)) ((out-l : 16) (out-r : 16))
  [in-l & in-r = split 16 in]
  [in-l -> out-l]
  [in-r -> out-r])
(splitter32)

;; (define (splitter32-man)
;;   (define c (default-component
;;               'splitter32
;;               (list (port 'in 32))
;;               (list (port 'left-out 16) (port 'right-out 16))))
;;   (split! c 'in 16 'in-left 'in-right)
;;   (connect! c 'in-left 'inf# 'left-out 'inf#)
;;   (connect! c 'in-right 'inf# 'right-out 'inf#)
;;   c)
;; (splitter32)

(define/module joiner32 ((in-l : 16) (in-r : 16)) ((out : 32))
  [out-l & out-r = split 16 out]
  [in-l -> out-l]
  [in-r -> out-r])
(joiner32)

;; (define (joiner32)
;;   (define c (default-component
;;               'joiner32
;;               (list (port 'in-left 16) (port 'in-right 16))
;;               (list (port 'out 32))))
;;   (split-out! c 'out 16 'out-left 'out-right)
;;   (connect! c 'in-left 'inf# 'out-left 'inf#)
;;   (connect! c 'in-right 'inf# 'out-right 'inf#)
;;   c)
;; (joiner32)

;; (plot (splitter32))
;; (plot (splitter32))
;; (plot (myadd-man))
;; (plot (myadd-man))

;; (define/module test (a b c) (x)
;;   ([cat = new myadd]
;;    [dog = new myadd]
;;    [a -> cat @ l]
;;    [b -> cat @ r]
;;    [cat @ o -> dog @ l]
;;    [c -> dog @ r]
;;    [dog @ o -> x]))
;; (test)
;; (plot (test))
