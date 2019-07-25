#lang racket/base

(require rackunit
         "../src/futil.rkt")

(define (check-compute comp inputs expected)
  (let* ([res (ast-tuple-state (compute comp inputs))])
    (for-each (lambda (x)
                (check equal?
                       (hash-ref res `(,(car x) . inf#))
                       (cdr x)))
              expected)))

(define/module decr ((in : 32)) ((out : 32))
  ([sub = new comp/trunc-sub]
   [const one 1 : 32 -> sub @ right]
   [in -> sub @ left]
   [sub @ out -> out])
  [])

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

(test-case
    "Simple computation"
  (check-compute (decr) '((in . 10)) '((out . 9))))

(test-case
    "ifen works"
  (define/module ifen-test ((in : 32)) ((out : 32))
    ([in -> out]
     [const one 1 : 32 -> out])
    [(ifen (in inf#)
           ([(one)])
           ([]))])
  (check-compute (ifen-test) '((in . 42)) '((out . 42)))
  (check-compute (ifen-test) '((in . #f)) '((out . 1))))

(test-case
    "Inputs retain values after disable and then reenable"
  (define/module input-retain ((a : 32)) ((out-1 : 32) (out-2 : 32))
    ([a -> out-1]

     [reg = new comp/reg]
     [a -> reg @ in]
     [reg @ out -> out-2])
    []
    [(a)]
    [])
  (check-compute (input-retain)
                 '((a . 10))
                 '((out-1 . 10)
                   (out-2 . 10))))

(test-case
    "Memory in submodules works correctly"
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
       ([(b zero)]))])
  (check-compute (mult) '((a . 8) (b . 7)) '((out . 56)))
  (check-compute (mult) '((a . 7) (b . 8)) '((out . 56)))
  (check-compute (mult) '((a . 12) (b . 4)) '((out . 48))))

(test-case
    "Addressable memory works"
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
  (check-compute (mem-test)
                 '((addr1 . 1)
                   (data1 . 6)
                   (addr2 . 2)
                   (data2 . 7))
                 '((out1 . 6)
                   (out2 . 7))))

(test-case
    "If statment works"
  (define/module if-test ((a : 32) (b : 32) (c : 32)) ((out : 32))
    ([a -> out]
     [b -> out])
    [(if (c inf#)
         ([(b)])
         ([(a)]))])
  (check-compute (if-test) '((a . 42) (b . 24) (c . 0)) '((out . 24)))
  (check-compute (if-test) '((a . 42) (b . 24) (c . 1)) '((out . 42)))
  (check-compute (if-test) '((a . 42) (b . 24) (c . #f)) '((out . #f))))
