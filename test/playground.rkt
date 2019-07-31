#lang racket/base
(require "../src/futil.rkt"
         "../src/vizualizer.rkt")

;; (require "futil.rkt")
;; (require "vizualizer.rkt")

(define/module incr ((in : 32)) ((out : 32))
  ([add = new comp/add]
   [const one 1 : 32 -> add @ left]
   [in -> add @ right]
   [add @ out -> out])
  [])

(define/module decr ((in : 32)) ((out : 32))
  ([sub = new comp/trunc-sub]
   [const one 1 : 32 -> sub @ right]
   [in -> sub @ left]
   [sub @ out -> out])
  [])

(define/module counter ((in : 32) (en : 32)) ((out : 32))
  ([sub = new comp/trunc-sub]
   [reg = new comp/reg]
   [con = new comp/id]
   [dis = new comp/id]

   [in -> sub @ left]
   [const decr 1 : 32 -> sub @ right]
   [sub @ out -> reg @ in]
   [reg @ out -> con @ in]
   [reg @ out -> dis @ in]

   [con @ out -> sub @ left]
   [sub @ out -> out]

   [dis @ out -> out])
  [(ifen (en inf#)
         ([(ifen (in inf#)
                 ([(con dis)])
                 ([(dis)]))])
         ([(in con)]))])

;; (require "vizualizer.rkt")
;; (plot-compute (counter) '((in . 10) (en . 1))
;;               #:memory (hash
;;                         'decr
;;                         (mem-tuple #f '#hash())
;;                         'in
;;                         (mem-tuple #f '#hash())
;;                         'out
;;                         (mem-tuple #f '#hash())
;;                         'reg
;;                         (mem-tuple 8 '#hash())
;;                         'sub
;;                         (mem-tuple #f '#hash())))

(define/module consumer ((n : 32)) ((out : 32))
  ([counter = new counter]
   [viz = new comp/id]
   [const en 1 : 32 -> counter @ en]
   [n -> counter @ in]
   [counter @ out -> viz @ in]
   [const on 1 : 32 -> out])
  [(on)]
  [(while (counter out)
     ([(n on)]))]
  [])
;; (plot-compute (consumer) '((n . 10)))

;; (plot-compute (consumer) '((n . 10)))

(define/module mult ((a : 32) (b : 32)) ((out : 32))
  ([counter = new counter]
   [add = new comp/add]
   [reg = new comp/reg]
   [viz = new comp/id]
   [con = new comp/id]

   [b -> counter @ in]
   [const en 1 : 32 -> counter @ en]
   [counter @ out -> viz @ in]

   [const zero 0 : 32 -> add @ left]
   [a -> add @ right]
   [add @ out -> reg @ in]
   [reg @ out -> con @ in]
   [con @ out -> add @ left]
   [add @ out -> out])
  [(con)]
  [(while (counter out)
     ([(b zero)]))])

;; (while (counter out) ([(b zero)]))
;; (ast-tuple-state (compute (mult) '((a . 7) (b . 8))))
;; (unlisten-debug)

(define/module mem-test ((addr1 : 32) (data1 : 32) (addr2 : 32) (data2 : 32)) ((out1 : 32) (out2 : 32))
  ([mem = new comp/memory-8bit]
   [addr1 -> mem @ addr]
   [addr2 -> mem @ addr]
   [data1 -> mem @ data-in]
   [data2 -> mem @ data-in]

   [viz = new comp/id]
   [mem @ out -> viz @ in]

   [reg1 = new comp/reg]
   [viz @ out -> reg1 @ in]
   [reg1 @ out -> out1]

   [reg2 = new comp/reg]
   [viz @ out -> reg2 @ in]
   [reg2 @ out -> out2])

  [(mem viz)]

  [(data1 addr2 data2 reg1 reg2)]
  [(addr2 data2 reg1 reg2)]
  [(data1 addr1 data2 reg1 reg2)]
  [(data1 addr1 reg1 reg2)]

  [(mem viz)]

  [(data1 addr2 data2 reg2)]
  [(data1 addr1 data2 reg1)]

  [(mem viz)])

;; (unlisten-debug)
;; (plot-compute (mem-test) '((addr1 . 1)
;;                            (data1 . 6)
;;                            (addr2 . 2)
;;                            (data2 . 7)))

(define/module counter-up-3out ((n : 32) (en : 32)) ((out1 : 32) (out2 : 32) (out3 : 32) (stop : 32))
  ([counter = new counter]
   [store-n = new comp/reg]

   [n -> store-n @ in]
   [n -> counter @ in]
   [en -> counter @ en]
   [sub = new comp/sub]
   [store-n @ out -> sub @ left]
   [counter @ out -> sub @ right]

   [decr1 = new decr]
   [decr2 = new decr]
   [decr3 = new decr]

   [counter @ out -> stop]

   [sub @ out -> decr1 @ in]
   [decr1 @ out -> out1]
   [decr1 @ out -> decr2 @ in]
   [decr2 @ out -> out2]
   [decr2 @ out -> decr3 @ in]
   [decr3 @ out -> out3])
  [(ifen (en inf#)
       ([(halt)])
       ([]))])

;; (plot-compute (counter-up-3out) '((n . 10) (en . 1)))

;; (plot-compute (test-c) '((n . 10)))

(define/module test-c ((n : 32)) ((out1 : 32) (out2 : 32) (out3 : 32))
  ([counter = new counter-up-3out]
   [const en 1 : 32 -> counter @ en]
   [n -> counter @ n]
   [counter @ out1 -> out1]
   [counter @ out2 -> out2]
   [counter @ out3 -> out3])
  []
  [(n)]
  [(n)]
  [(n en)]
  [(n en)]
  [(n)]
  [(n)])


(define/module fib ((n : 32)) ((out : 32))
  ([mem = new comp/memory-8bit]
   [counter = new counter-up-3out]
   [incr = new incr]
   [n -> incr @ in]
   [incr @ out -> counter @ n]

   [add = new comp/add]

   [con1 = new comp/id]
   [con2 = new comp/id]
   [con3 = new comp/id]
   [con4 = new comp/id]
   [con5 = new comp/id]
   [const en 1 : 32 -> counter @ en]
   [counter @ out1 -> con1 @ in]
   [counter @ out2 -> con2 @ in]
   [counter @ out3 -> con3 @ in]

   [const fib0 0 : 32 -> mem @ addr]
   [const fib1 1 : 32 -> mem @ addr]
   [con1 @ out -> mem @ addr]
   [con2 @ out -> mem @ addr]
   [con3 @ out -> mem @ addr]

   [lreg = new comp/reg]
   [rreg = new comp/reg]
   [lreg @ out -> add @ left]
   [rreg @ out -> add @ right]

   [mem @ out -> con4 @ in]
   [mem @ out -> con5 @ in]
   [con4 @ out -> lreg @ in]
   [con5 @ out -> rreg @ in]

   [const one 1 : 32 -> mem @ data-in]
   [add @ out -> mem @ data-in]

   [mem @ out -> out])
  [(con1 con2 con3 con4 con5 fib1 add)]
  [(n incr con1 con2 con3 con4 con5 fib0 add)]
  [(n incr one fib0 fib1 add con1 con2 con3 con4 con5 mem)]

  [(while (counter stop)
     ([(en n incr one fib0 fib1 add con1 con2 con5)]
      [(en n incr one fib0 fib1 add con1 con3 con4)]
      [(en n incr one fib0 fib1 con2 con3 con4 con5)]
      [(n incr one fib0 fib1 add con1 con2 con3 con4 con5 mem)]
      ))])

;; (unlisten-debug)
;; (plot-compute (fib) '((n . 30)))
;; (component-outs (counter))

