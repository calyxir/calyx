#lang racket/base

(require futil
         futil/visualizer)

(define/module counter ((in : 32)) ((out : 32))
  ([sub = new comp/trunc-sub]
   [reg = new comp/reg]
   [in -> reg @ in]
   [reg @ out -> sub @ left]
   [const decr 1 : 32 -> sub @ right]
   [sub @ out -> out]
   [sub @ out -> reg @ in])
  [(ifen (in) ([]) ())]
  [(in)])

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
   [add @ out -> out])
  []
  [(while (counter @ out) ([(b zero)]))])

(plot-compute (mult) '((a . 3) (b . 5)))
