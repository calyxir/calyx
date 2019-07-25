#lang racket/base
(require "../src/futil.rkt"
         "../src/vizualizer.rkt")

(define/module decr ((in : 32)) ((out : 32))
  ([sub = new comp/trunc-sub]
   [const one 1 : 32 -> sub @ right]
   [in -> sub @ left]
   [sub @ out -> out])
  [])

;; (ast-tuple-state (compute (decr) '((in . 1))))
(define/module counter ((in : 32)) ((out : 32))
  ([sub = new comp/trunc-sub]
   [reg = new comp/reg]
   [in -> sub @ left]
   [const decr 1 : 32 -> sub @ right]
   [sub @ out -> reg @ in]
   [reg @ out -> sub @ left]
   [reg @ out -> out])
  [(ifen (in inf#)
         ([])
         ([(in)]
          [(in)]))])
;; (plot-compute (counter) '((in . #f))
;;               #:memory (make-immutable-hash
;;                         `((reg . ,(mem-tuple 5 (make-immutable-hash)))))
;;          )

(define/module consumer ((n : 32)) ((out : 32))
  ([counter = new counter]
   [viz = new comp/id]
   [n -> counter @ in]
   [counter @ out -> viz @ in]
   [const on 1 : 32 -> out])
  [(on)]
  [(n on)]
  [(n on)]
  ;; [(while (counter out)
  ;;    ([(n on)]))]
  )
;; (listen-debug)
;; (plot-compute (consumer) '((n . 10)))

;; (plot-compute (consumer) '((n . 10)))

(define/module mult ((a : 32) (b : 32)) ((out : 32))
  ([counter = new counter]
   [add = new comp/add]
   [reg = new comp/reg]
   [viz = new comp/id]

   [b -> counter @ in]
   [counter @ out -> viz @ in]

   [const zero 0 : 32 -> add @ left]
   [a -> add @ right]
   [add @ out -> reg @ in]
   [reg @ out -> add @ left]
   [reg @ out -> out])
  []
  [(while (counter out)
     ([(b zero)]))]
  )
;; (while (counter out) ([(b zero)]))
;; (listen-debug)
;; (plot-compute (mult) '((a . 7) (b . 8)))
;; (unlisten-debug)

(define/module mem-test ((addr1 : 32) (data1 : 32) (addr2 : 32) (data2 : 32)) ((out : 32))
  ([mem = new comp/memory-8bit]
   [addr1 -> mem @ addr]
   [addr2 -> mem @ addr]
   [data1 -> mem @ data-in]
   [data2 -> mem @ data-in]

   [viz = new comp/id]
   [mem @ out -> viz @ in])

  [(mem)]

  [(data1 addr2 data2)]
  [(addr2 data2)]
  [(data1 addr1 data2)]
  [(data1 addr1)]

  [(mem)]

  [(data1 addr2 data2)]
  [(data1 addr1 data2)])

;; (unlisten-debug)
;; (plot-compute (mem-test) '((addr1 . 1)
;;                            (data1 . 6)
;;                            (addr2 . 2)
;;                            (data2 . 7)))
