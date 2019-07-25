#lang racket/base

(require racket/hash
         racket/bool
         racket/sequence
         racket/list
         racket/pretty
         racket/format
         racket/match
         racket/contract
         graph
         "component.rkt"
         "port.rkt")

(provide (struct-out par-comp)
         (struct-out seq-comp)
         (struct-out deact-stmt)
         (struct-out if-stmt)
         (struct-out ifen-stmt)
         (struct-out while-stmt)
         (struct-out ast-tuple)
         (struct-out mem-tuple)
         compute)

;; type of statements
(define-struct/contract par-comp
  ([stmts (and/c list? (not/c empty?))])
  #:transparent)

(define-struct/contract seq-comp
  ([stmts list?])
  #:transparent)

(define-struct/contract deact-stmt
  ([mods (listof symbol?)])
  #:transparent)

(define-struct/contract if-stmt
  ([condition pair?]
   [tbranch any/c]
   [fbranch any/c])
  #:transparent)

(define-struct/contract ifen-stmt
  ([condition pair?]
   [tbranch any/c]
   [fbranch any/c])
  #:transparent)

(define-struct/contract while-stmt
  ([condition pair?]
   [body any/c])
  #:transparent)

;; a hash union that tries to make overlapping keys non-false
;;   if v1 or v2 is #f, choose non-false option
;;   otherwise, if both v1 and v2 have values, choose v2
(define (save-hash-union h1 h2)
  (hash-union
   h1
   h2
   #:combine (lambda (v1 v2) (if (and v1 v2) v2 (xor v1 v2)))))

;; a hash union function that always prefers h2 when keys overlap
(define (clob-hash-union h1 h2)
  (hash-union h1 h2 #:combine (lambda (v1 v2) v2)))

;; a hash union function that chooses non-false values
;; over false ones, keeps equal values the same,
;; and errors on non-equal values
(define (equal-hash-union h0 h1
                          #:error [error-msg "Expected same values or one false."])
  (hash-union
   h0
   h1
   #:combine
   (lambda (v0 v1)
     (cond
       [(xor v0 v1) (or v1 v0)] ; when only one is false, choose the true one.
       [(equal? v0 v1) v0]      ; v0 = v1, then v0
       [else
        (raise-result-error 'equal-hash-union error-msg `(,h0 ,h1))]))))

(define (input-hash comp lst)
  (define empty-hash
    (make-immutable-hash
     (map (lambda (x) `(,x . #f))
          (append
           (map car (get-edges (component-graph comp)))
           (map (lambda (p)
                  `(,(port-name p) . inf#))
                (component-outs comp))))))

  (clob-hash-union
   empty-hash
   (make-immutable-hash
    (map (lambda (x) `((,(car x) . inf#) . ,(cdr x))) lst))))

(define (transform comp inputs name)
  (if (findf (lambda (x) (equal? name (port-name x))) (component-ins comp))
      (make-immutable-hash `(((,name . inf#) . ,(hash-ref inputs `(,name . inf#)))))
      (begin
        (let* ([sub (get-submod! comp name)]
               [ins (map port-name (component-ins sub))])  ; XXX: deal with port widths
          (make-immutable-hash
           (map (lambda (in)
                  (define neighs
                    (sequence->list
                     (in-neighbors
                      (transpose (component-graph comp)) `(,name . ,in))))
                  (define filt-neighs-vals
                    (filter-map (lambda (x) (hash-ref inputs x)) neighs))
                  (define neighs-vals
                    (if (empty? filt-neighs-vals)
                        (map (lambda (x) (hash-ref inputs x)) neighs)
                        filt-neighs-vals))
                  `((,name . ,in) . ,(car neighs-vals)))
                ins))))))

; (submod -> mem-tuple) hash
; mem-tuple = (value * (submod -> mem-tuple) hash)
(struct mem-tuple (value sub-mem) #:transparent)
(define (empty-mem-tuple) (mem-tuple #f (make-immutable-hash)))

;; given a subcomponent (comp name) a state and memory,
;; run subcomponents proc with state and memory and
;; return updated state and memory
(define (submod-compute comp name state mem-tup)
  ;; state is of the form (((sub . port) . val) ...)
  ;; change to ((port . val) ...)
  (define ins
    (make-immutable-hash
     (hash-map state (lambda (k v) `(,(cdr k) . ,v)))))

  ;; add sub-memory and memory value to ins
  (define ins-p (hash-set* ins
                           'sub-mem# (mem-tuple-sub-mem mem-tup)
                           'mem-val# (mem-tuple-value mem-tup)))

  (let* ([proc (component-proc (get-submod! comp name))]
         [mem-proc (component-memory-proc (get-submod! comp name))]
         [state-res (proc ins-p)]
         [sub-mem-p (hash-ref state-res 'sub-mem#
                              (make-immutable-hash))]
         [state-wo-mem (hash-remove state-res 'sub-mem#)]
         [value-p (mem-proc (mem-tuple-value mem-tup)
                          (save-hash-union ins state-wo-mem))]
         [mem-tup-p (mem-tuple value-p sub-mem-p)])
    (values
     (make-immutable-hash
      (hash-map state-wo-mem
                (lambda (k v) `((,name . ,k) . ,v))))
     mem-tup-p)))

(define (compute-step comp memory state inactive)
  ;; sort the components so that we evaluate things in the right order
  (define order (tsort (convert-graph comp)))

  ;; function that goes through a given hashmap and sets all disabled wires to false
  (define (filt hsh)
    (make-immutable-hash
     (hash-map hsh (lambda (k v) (if (member (car k) inactive)
                                     `(,k . #f)
                                     `(,k . ,v))))))

  ;; for every node in the graph, call submod-compute;
  ;; making sure to thread the state through properly
  (struct accum (state memory))
  (define filled
    (foldl (lambda (sub acc)
             (define res
               (if (member sub inactive)
                   ; inactive (do nothing)
                   (begin (log-debug "here") acc)
                   ; active
                   (let*-values
                       ([(mem-tup) (hash-ref (accum-memory acc) sub
                                             (lambda () (empty-mem-tuple)))]
                        ; remove disabled wires from memory, then union with state
                        [(state) (filt (accum-state acc))]
                        [(trans) (transform comp state sub)]
                        [(outs mem-tup-p) ; pass in state and memory to submodule
                         (submod-compute comp sub trans mem-tup)]
                        [(state-p) (save-hash-union (accum-state acc) outs)])
                     (log-debug "mem-val (~v): ~v" sub mem-tup-p)
                     (log-debug "state-p(~v): ~v" sub state-p)
                     (accum
                      state-p
                      (hash-set (accum-memory acc) sub mem-tup-p)))))
             res)
           (accum state memory)
           order))

  ;; after we have used all the values, set the wires coming from inactive modules to #f
  (define filled-mod
    (foldl (lambda (x acc)
             (hash-set acc x #f))
           (accum-state filled)
           (filter (lambda (x) (member (car x) inactive))
                   (hash-keys (accum-state filled)))))

  ;; (define filled-mod filled)
  (values
   filled-mod                ; state
   (accum-memory filled)     ; memory
   (map (lambda (x)          ; output
          `(,(car x) . ,(hash-ref (accum-state filled) x)))
        (map (lambda (x) `(,(port-name x) . inf#)) (component-outs comp)))))

(define-syntax-rule (if-valued condition tbranch fbranch disbranch)
  (if condition
      (if (not (equal? condition 0))
          tbranch
          fbranch)
      disbranch))

(struct ast-tuple (inputs inactive state memory history) #:transparent)

(define (merge-state st0 st1)
  (equal-hash-union st0 st1))

(define (update-history ast-tup)
  (struct-copy ast-tuple ast-tup
               [history (cons ast-tup (ast-tuple-history ast-tup))]))

(define (ast-step comp tup ast)
  (match-define (ast-tuple inputs inactive state memory history) tup)
  (log-debug "(open ast-step ~v" ast)
  (define result
    (match ast
      [(par-comp stmts)
       (define (merge-tup tup1 tup2)
         (match-let ([(ast-tuple ins-1 inact-1 st-1 mem-1 hist-1)
                      tup1]
                     [(ast-tuple ins-2 inact-2 st-2 mem-2 hist-2)
                      tup2])
           (ast-tuple
            inputs
            (remove-duplicates (append inact-1 inact-2))
            (merge-state st-1 st-2)
            mem-1 ;; XXX fix this
            history)))
       (foldl merge-tup
              (struct-copy ast-tuple tup
                           [state (make-immutable-hash)]
                           [memory (make-immutable-hash)])
              (map (lambda (s) (ast-step comp tup s))
                   stmts))]
      [(seq-comp stmts)
       (foldl (lambda (s acc)
                (define acc-p (struct-copy ast-tuple acc
                                           [inactive (ast-tuple-inactive tup)]))
                (define res (ast-step comp acc-p s))
                (update-history res))
              tup
              stmts)]
      [(deact-stmt mods) ; compute step with this list of inactive modules
       (let*-values ([(st mem out)
                      (compute-step comp
                                    memory
                                    (save-hash-union inputs state)
                                    mods)])
         (log-debug "state: ~v\n memory: ~v\n" st mem)
         (struct-copy ast-tuple tup
                      [state st]
                      [memory mem]
                      [inactive mods]))]
      [(if-stmt condition tbranch fbranch)
       (if-valued (hash-ref state condition)
                  (ast-step comp tup tbranch)
                  (ast-step comp tup fbranch)
                  tup)]
      [(ifen-stmt condition tbranch fbranch)
       (if (hash-ref state condition)
           (ast-step comp tup tbranch)
           (ast-step comp tup fbranch))]
      [(while-stmt condition body)
       (if-valued (hash-ref state condition)
                  (ast-step comp
                            (ast-step comp tup body)
                            ast)
                  tup
                  tup)]
      [#f (ast-step comp tup (deact-stmt '()))]
      [_ (error "Malformed ast!" ast)]))
  (log-debug "close)")
  result)

(define (compute comp inputs #:memory [mem (make-immutable-hash)])
  (define ast (component-control comp))
  (define state (input-hash comp inputs))
  (log-debug "================")
  (log-debug "(start compute for ~v" (component-name comp))
  (log-debug "memory: ~v" mem)
  (define result (ast-step comp (ast-tuple state '() state mem '()) ast))

  (log-debug "~v" (ast-tuple-state result))
  (log-debug "~v" (ast-tuple-memory result))
  (log-debug "end compute)")
  (log-debug "================")
  result)
