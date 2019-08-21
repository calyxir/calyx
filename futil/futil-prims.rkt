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
                    [out => mem-val#])
    #:memory-proc (lambda (old st)
                    (define new-v (hash-ref st 'in))
                    (if new-v new-v old))
    #:time-increment 1))

(define (comp/res-reg)
  (default-component
    'res-reg
    (list (port 'in 32)
          (port 'res 32))
    (list (port 'out 32))
    (keyword-lambda (mem-val# in res) ()
                    [out =>
                         (if res
                             #f
                             (if in in mem-val#))])
    #:memory-proc (lambda (old st)
                    (define new-v (hash-ref st 'in))
                    (define res (hash-ref st 'res))
                    (if res
                        #f
                        (if new-v new-v old)))
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
                     ;; [addr = (~a addr1 'x addr2)]
                     [addr = (cons addr1 addr2)]
                     )
                    [out => (if data-in
                                data-in
                                (hash-ref mem addr
                                          (lambda () #f)))])
    #:memory-proc (lambda (old st)
                    (let* ([hsh (if (hash? old) old (make-immutable-hash))]
                           [addr1 (hash-ref st 'addr1)]
                           [addr2 (hash-ref st 'addr2)]
                           [data-in (hash-ref st 'data-in)]
                           ;; [addr (~a addr1 'x addr2)]
                           [addr (cons addr1 addr2)])
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
(define (comp/sqrt)
  (default-component
    'sqrt
    (list (port 'in 32))
    (list (port 'out 32))
    (keyword-lambda (in) ()
                    [out => (if in (sqrt in) #f)])))

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

(define/module and3way ((a : 32) (b : 32) (c : 32)) ((out : 32))
  ([const en 1 : 32 -> out])
  [(ifen (a)
         ([(ifen (b)
                 ([(ifen (c)
                         ([(!! en out)])
                         ())])
                 ())])
         ())])

(define/module comp/iterator
  ((start : 32) (incr : 32) (end : 32) (en : 32))
  ((out : 32) (stop : 32))
  ([incr-reg = new comp/reg]
   [end-reg = new comp/reg]
   [add = new comp/add]
   [cmp = new comp/trunc-sub]

   [ins-and = new and3way]
   [start -> ins-and @ a]
   [incr -> ins-and @ b]
   [end -> ins-and @ c]

   [incr -> incr-reg @ in]
   [end -> end-reg @ in]

   [val-reg = new comp/res-reg]
   [const res-val 1 : 32 -> val-reg @ res]

   [const add-zero 0 : 32 -> add @ right]
   [start -> add @ left]
   [incr-reg @ out -> add @ right]
   [add @ out -> val-reg @ in]
   [val-reg @ out -> add @ left]
   [add @ out -> out]
   [end-reg @ out -> cmp @ left]
   [add @ out -> cmp @ right]
   [cmp @ out -> stop])
  [(!! start incr end ins-and)]
  [(ifen (en)
         ([(ifen (ins-and @ out)
                 ([(!! res-val val-reg)]
                  [(!! start incr end incr-reg end-reg)]
                  [(incr incr-reg end res-val)])
                 ([(add-zero start incr end res-val)]))])
         ([(start incr incr-reg end res-val)]))])
