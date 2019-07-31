#lang racket/base

(require racket/hash
         racket/bool
         racket/sequence
         racket/list
         racket/pretty
         racket/format
         racket/match
         racket/contract
         racket/set
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
   #:combine (lambda (v1 v2)
               (cond [(and v1 v2) v2]
                     ;; [(and v1 v2) (error (format "Couldn't merge: ~v & ~v\n~v\n~v"
                     ;;                             v1 v2
                     ;;                             h1 h2))]
                     [else (xor v1 v2)]))))

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

(struct stamped (val t) #:transparent)

; XXX factor with transform
(define (restrict-inputs comp state name)
  (define sub (get-submod! comp name))
  (define ins (map port-name (component-ins sub)))
  (foldl (lambda (in acc)
           (define neighs
             (sequence->list
              (in-neighbors
               (transpose (component-graph comp)) `(,name . ,in))))
           (foldl (lambda (x acc)
                    (hash-set acc x (hash-ref state x)))
                  acc
                  neighs))
         (make-immutable-hash)
         ins))

(define (transform comp inputs name)
  (if (findf (lambda (x) (equal? name (port-name x))) (component-ins comp))
      ; if name is an input, (((in . inf#) . v) ...) -> ((in . inf#) . v)
      (make-immutable-hash `(((,name . inf#) . ,(hash-ref inputs `(,name . inf#)))))
      ; else name is not an input
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
                    (filter-map (lambda (x)
                                  (define stamp (hash-ref inputs x))
                                  (if (stamped-val stamp)
                                      stamp
                                      #f))
                                neighs))
                  (define neighs-val
                    (match filt-neighs-vals
                      [(list) (stamped #f 0)]
                      [(list x) x]
                      [x (error "Overlapping values!" x)]))
                  `((,name . ,in) . ,neighs-val))
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
  (define in-vals
    (make-immutable-hash
     (hash-map state (lambda (k v) `(,(cdr k) . ,v)))))

  ;; add sub-memory and memory value to in-vals
  (define in-vals-p (hash-set* in-vals
                           'sub-mem# (mem-tuple-sub-mem mem-tup)
                           'mem-val# (mem-tuple-value mem-tup)))

  (let* ([sub (get-submod! comp name)]
         [proc (component-proc sub)]
         [mem-proc (component-memory-proc sub)]
         [state-res (proc in-vals-p)]
         [sub-mem-p (hash-ref state-res 'sub-mem#
                              (make-immutable-hash))]
         [state-wo-mem (hash-remove state-res 'sub-mem#)]
         [value-p (mem-proc (mem-tuple-value mem-tup)
                            (save-hash-union in-vals state-wo-mem))]
         [mem-tup-p (mem-tuple value-p sub-mem-p)])
    (values
     (make-immutable-hash
      (hash-map state-wo-mem
                (lambda (k v) `((,name . ,k) . ,v))))
     mem-tup-p)))

(define-syntax-rule (if-valued condition tbranch fbranch disbranch)
  (if condition
      (if (not (equal? condition 0))
          tbranch
          fbranch)
      disbranch))

(struct ast-tuple (inputs inactive state memory) #:transparent)

(define (compute-step comp tup)
  (log-debug "compute-step ~a" (ast-tuple-state tup))
  (log-debug "inactives mods: ~a" (ast-tuple-inactive tup))
  (define (filt tup lst)
    (define state (ast-tuple-state tup))
    (struct-copy ast-tuple tup
                 [state
                  (make-immutable-hash
                   (hash-map state
                             (lambda (k v)
                               (match-define (stamped val t) v)
                               (if (member (car k) lst)
                                   `(,k . ,(stamped #f t))
                                   `(,k . ,v)))))]))

  (define (stamp state)
    (make-immutable-hash
     (hash-map state (lambda (k v) `(,k . ,(stamped v 0))))))

  (define (stamp-tup tup)
    (struct-copy ast-tuple tup
                 [state (stamp (ast-tuple-state tup))]))

  (define (unstamp state)
    (make-immutable-hash
     (hash-map state (lambda (k v) `(,k . ,(stamped-val v))))))

  (define (unstamp-tup tup)
    (struct-copy ast-tuple tup
                 [state (unstamp (ast-tuple-state tup))]))

  (define (worklist tup todo visited)
    (log-debug "worklist todo: ~a" todo)
    (cond [(empty? todo) tup]
          [else
           (define name (car todo))
           (match-define (ast-tuple _ inactive state memory) tup)

           (define ts-valid?
             (apply =
                    (append '(0 0)
                            (hash-map (restrict-inputs comp state name)
                                      (lambda (k v) (stamped-t v))))))
           (log-debug "ts-valid? ~a: ~a" name ts-valid?)

           (define-values (tup-p todo-p)
             (cond [(member name inactive) ; inactive
                    (log-debug "~a inactive" name)
                    (values (filt tup inactive) (cdr todo))]
                   [(not ts-valid?)
                    (values tup (cdr todo))]
                   [else ; active
                    (let*-values
                        ([(trans) (transform comp state name)]
                         [(mem-tup) (hash-ref memory name empty-mem-tuple)]
                         [(outs mem-tup-p)
                          (submod-compute comp name (unstamp trans) mem-tup)]
                         [(time-incr) (component-time-increment (get-submod! comp name))]
                         [(outs-p)
                          (if (set-member? visited name)
                              (make-immutable-hash
                               (hash-map
                                outs
                                (lambda (k v)
                                  `(,k . ,(stamped v time-incr)))))
                              (stamp outs))]
                         [(debug)
                          (begin
                            (log-debug "transformed ~a: ~a" name trans)
                            (log-debug "result ~a: ~a" name outs-p))]
                         [(state-p) (save-hash-union state outs-p)]
                         [(tup-p)
                          (struct-copy ast-tuple tup
                                       [state state-p]
                                       [memory (hash-set memory name mem-tup-p)])]
                         [(todo-p)
                          (remove-duplicates
                           (append
                            (cdr todo)
                            (sequence->list (in-neighbors (convert-graph comp) name))))])
                      (values tup-p todo-p))]))

           (worklist tup-p todo-p (set-add visited name))]))

  (define res
    (unstamp-tup
     (worklist (stamp-tup tup)
               (tsort (convert-graph comp))
               (set))))

  (values
   (ast-tuple-state res)
   (ast-tuple-memory res)))

(define (merge-state st0 st1)
  (equal-hash-union st0 st1))

(define (ast-step comp tup ast #:hook [callback void])
  (match-define (ast-tuple inputs inactive state memory) tup)
  (log-debug "(open ast-step ~v" ast)
  (define result
    (match ast
      [(par-comp stmts)
       (define (merge-tup tup1 tup2)
         (match-let ([(ast-tuple ins-1 inact-1 st-1 mem-1)
                      tup1]
                     [(ast-tuple ins-2 inact-2 st-2 mem-2)
                      tup2])
           (ast-tuple
            inputs
            (remove-duplicates (append inact-1 inact-2))
            (merge-state st-1 st-2)
            mem-1 ;; XXX fix this
            )))
       (foldl merge-tup
              (struct-copy ast-tuple tup
                           [state (make-immutable-hash)]
                           [memory (make-immutable-hash)])
              (map (lambda (s) (ast-step comp tup s #:hook callback)) stmts))]
      [(seq-comp stmts)
       (foldl (lambda (s acc)
                (define acc-p (struct-copy ast-tuple acc
                                           [inactive (ast-tuple-inactive tup)]))
                (ast-step comp acc-p s #:hook callback))
              tup
              stmts)]
      [(deact-stmt mods) ; compute step with this list of inactive modules
       (let*-values ([(tup-p)
                      (struct-copy ast-tuple tup
                                   [state (save-hash-union inputs state)]
                                   [inactive mods] ;; [inactive (remove-duplicates (append inactive mods))]
                                   )]
                     [(st mem)
                      (compute-step comp tup-p)])
         (log-debug "state: ~v" st)
         (struct-copy ast-tuple tup
                      [state st]
                      [memory mem]
                      [inactive mods]))]
      [(if-stmt condition tbranch fbranch)
       (if-valued (hash-ref state condition)
                  (ast-step comp tup tbranch #:hook callback)
                  (ast-step comp tup fbranch #:hook callback)
                  tup)]
      [(ifen-stmt condition tbranch fbranch)
       (if (hash-ref state condition)
           (ast-step comp tup tbranch #:hook callback)
           (ast-step comp tup fbranch #:hook callback))]
      [(while-stmt condition body)
       (if-valued (hash-ref state condition)
                  (let* ([bodyres (ast-step comp tup body #:hook callback)]
                         [res (ast-step comp bodyres ast #:hook callback)])
                    res)
                  tup
                  tup)]
      [#f (ast-step comp tup (deact-stmt '()) #:hook callback)]
      [_ (error "Malformed ast!" ast)]))
  (log-debug "close)")
  (callback result)
  result)

(define (compute comp inputs #:memory [mem (make-immutable-hash)] #:hook [callback void])
  (define ast (component-control comp))
  (define state (input-hash comp inputs))
  (log-debug "================")
  (log-debug "(start compute for ~v" (component-name comp))
  (define result (ast-step comp (ast-tuple state '() state mem) ast #:hook callback))

  (log-debug "~v" (ast-tuple-state result))
  (log-debug "~v" (ast-tuple-memory result))
  (log-debug "end compute)")
  (log-debug "================")
  result)
