#lang racket/base

(require racket/format
         "component.rkt"
         "port.rkt"
         "util.rkt"
         "futil-syntax.rkt")

(provide (all-defined-out))

(define input-list
  (list (port 'left 32)
        (port 'right 32)))
(define output-list
  (list (port 'out 32)))

(define-syntax-rule (falsify-apply op item ...)
  (if (andmap (lambda (x) x) (list item ...))
      (apply op (list item ...))
      #f))

(define (simple-binop name op)
  (default-component
    name
    input-list
    output-list
    (keyword-lambda (left right) ()
                    [out => (falsify-apply op left right)])))

(define (comp/id)
  (default-component
    'id
    (list (port 'in 32))
    (list (port 'out 32))
    (keyword-lambda (in) ()
                    [out => in])))

(define (comp/reg)
  (default-component
    'reg
    (list (port 'in 32))
    (list (port 'out 32))
    (keyword-lambda (mem-val# in) ()
                    [out => (if in in mem-val#)])
    #:memory-proc (lambda (old st)
                    (define new-v (hash-ref st 'in))
                    (if new-v new-v old))
    #:time-increment 1))

(define (comp/memory1d)
  (default-component
    'mem-1d
    (list (port 'addr 32)     ; XXX should be 8 bits
          (port 'data-in 32))
    (list (port 'out 32))
    (keyword-lambda (mem-val# addr data-in)
                    ([mem = (if (hash? mem-val#) mem-val# (make-immutable-hash))])
                    [out => (if data-in
                                data-in
                                (hash-ref mem addr
                                          (lambda () #f)))])
    #:memory-proc (lambda (old st)
                    (let* ([hsh (if (hash? old) old (make-immutable-hash))]
                           [data-in (hash-ref st 'data-in)]
                           [addr (hash-ref st 'addr)])
                      (if (and addr data-in)
                          (hash-set hsh addr data-in)
                          hsh)))))

(define (comp/memory2d)
  (default-component
    'mem-2d
    (list (port 'addr1 32)
          (port 'addr2 32)
          (port 'data-in 32))
    (list (port 'out 32))
    (keyword-lambda (mem-val# addr1 addr2 data-in)
                    ([mem = (if (hash? mem-val#) mem-val# (make-immutable-hash))]
                     [addr = (~a addr1 'x addr2)])
                    [out => (if data-in
                                data-in
                                (hash-ref mem addr
                                          (lambda () #f)))])
    #:memory-proc (lambda (old st)
                    (let* ([hsh (if (hash? old) old (make-immutable-hash))]
                           [addr1 (hash-ref st 'addr1)]
                           [addr2 (hash-ref st 'addr2)]
                           [data-in (hash-ref st 'data-in)]
                           [addr (~a addr1 'x addr2)])
                      (if (and data-in addr1 addr2)
                          (hash-set hsh addr data-in)
                          hsh)))))

(define (comp/trunc-sub)
  (default-component
    'trunc-sub
    input-list
    output-list
    (keyword-lambda (left right) ()
                    [out => (let ([x (falsify-apply - left right)])
                              (cond [(not x) #f]
                                    [(< x 0) 0]
                                    [else x]))])))

(define (comp/add) (simple-binop 'add +))
(define (comp/sub) (simple-binop 'sub -))
(define (comp/div) (simple-binop 'div /))
(define (comp/mult) (simple-binop 'mult *))
(define (comp/and) (simple-binop 'and bitwise-and))
(define (comp/or) (simple-binop 'or bitwise-ior))
(define (comp/xor) (simple-binop 'xor bitwise-xor))

(define (magic/mux)
  (default-component
    'mux
    (list (port 'left 32)
          (port 'right 32)
          (port 'control 1))
    (list (port 'out 32))
    (keyword-lambda (left right control) ()
                    [out => (if (= 1 control)
                                left
                                right)])))

(define/module comp/counter-down ((in : 32) (en : 32)) ((out : 32) (stop : 32))
  ([sub = new comp/trunc-sub]
   [reg = new comp/reg]
   [con = new comp/id]
   [dis = new comp/id]
   [in -> sub @ left]
   [const decr 1 : 32 -> sub @ right]
   [sub @ out -> reg @ in]
   [sub @ out -> out]
   [reg @ out -> con @ in]
   [reg @ out -> dis @ in]
   [dis @ out -> out]
   [con @ out -> sub @ left]

   [sub+1 = new comp/trunc-sub]
   [add = new comp/add]
   [reg1 = new comp/reg]
   [con1 = new comp/id]
   [dis1 = new comp/id]

   [in -> add @ left]
   [const a 1 : 32 -> add @ right]
   [add @ out -> sub+1 @ left]
   [const decr1 1 : 32 -> sub+1 @ right]
   [sub+1 @ out -> reg1 @ in]
   [sub+1 @ out -> stop]
   [reg1 @ out -> con1 @ in]
   [reg1 @ out -> dis1 @ in]
   [dis1 @ out -> stop]
   [con1 @ out -> sub+1 @ left])
  [(ifen (en)
         ([(ifen (in)
                 ([(con dis con1 dis1)])
                 ([(dis dis1)]))])
         ([(!! reg dis out reg1 dis1 stop)]))])

(define/module comp/counter-up ((in : 32) (en : 32)) ((out : 32) (stop : 32))
  ([counter = new comp/counter-down]
   [store-n = new comp/reg]
   [sub = new comp/trunc-sub]

   [en -> counter @ en]
   [in -> counter @ in]
   [in -> store-n @ in]

   [store-n @ out -> sub @ left]
   [counter @ stop -> sub @ right]
   [sub @ out -> out]
   [counter @ stop -> stop])
  [(ifen (in)
         ([(!! en in store-n counter)]
          [(en in sub)])
         ([]))])
