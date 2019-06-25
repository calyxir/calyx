#lang racket
(require graph
         racket/hash
         "dis-graphs.rkt"
         "port.rkt"
         "constraint.rkt")
(provide keyword-lambda
         (struct-out component)
         input-component
         output-component
         default-component
         make-constant
         connect!
         add-submod!
         get-submod!
         ;; add-in-hole!
         ;; add-out-hole!
         add-constraint!
         split!
         ;; get-neighs
         ;; follow-holes ; temp
         ;; compute
         ;; stabilize
         convert-graph
         plot)

(define-syntax-rule (keyword-lambda (arg ...)
                                    [kw = body ...] ...)
  (lambda (h)
    (define arg (hash-ref h 'arg)) ...
    (make-hash `((kw . ,(begin body ...)) ...))))

;; (struct s-hole (name    ;; name of the hole
;;                 pair    ;; the port that is connected to this
;;                 type)   ;; #t for input and #f for output
;;   #:transparent)
;; (define (left-handed? hole) (s-hole-type hole))
;; (define (right-handed? hole) (not (s-hole-type hole)))

(struct component (name                       ;; name of the component
                   [ins #:mutable]            ;; list of input ports
                   [outs #:mutable]           ;; list of output ports
                   submods                    ;; hashtbl of sub components keyed on their name
                   splits                     ;; hashtbl keeping track of split nodes
                   ;; holes                      ;; hashtbl from hole-names to holes
                   [constraints #:mutable]    ;; a list of constraints
                   proc                       ;; procedure representing this modules computation
                   graph                      ;; graph representing internal connections
                   primitive                  ;; true when this component is primitive
                   ))

;; creates a default component given a name for the component,
;; a list of input port names, and a list of output port names
(define (empty-graph) (weighted-graph/directed '()))

;; creates a component with a single infinite output port of width w
;; and no input ports. Designed to be used as the input of a component.
(define (input-component w) (component
                             'input
                             '()
                             (list (port 'inf# w))
                             (make-hash) ; submods
                             (make-hash) ; splits
                             ;; (make-hash) ; holes
                             '()
                             (keyword-lambda (inf#) [inf# = inf#])
                             (empty-graph)
                             #f))

;; creates a component with a single infinite input port of width w
;; and no output ports. Designed to be used as the output of a component.
(define (output-component w) (component
                              'output
                              (list (port 'inf# w))
                              '()
                              (make-hash) ; submods
                              (make-hash) ; splits
                              ;; (make-hash) ; holes
                              '()
                              (keyword-lambda (inf#) [inf# = inf#])
                              (empty-graph)
                              #f))

;; TODO: maybe add vertices for ins and outs

;; given a name, list of input ports, and list of output ports, creates
;; a component an empty graph and the appropriate input and output ports
;; in the hashtable.
(define (default-component name ins outs proc [prim #f])
  (let ([htbl (make-hash)]
        [g (empty-graph)])
    (for-each (lambda (p) ; p is a port
                (hash-set! htbl (port-name p) (input-component (port-width p))))
              ins)
    (for-each (lambda (p)
                (hash-set! htbl (port-name p) (output-component (port-width p))))
              outs)
    (component
     name
     ins
     outs
     htbl          ; sub-mods
     (make-hash)   ; splits
     ;; (make-hash)   ; holes
     '()
     proc
     g
     prim)))

(define (make-constant n width)
  (default-component n '() (list (port 'inf# width)) (keyword-lambda () [out = n]) #t))

;; Looks for an input/output port matching [port] in [comp]. If the port is found
;; and is equal to the value [#f], then this function does nothing. Otherwise
;; it removes that port from [comp].
(define (consume! comp port set-prop! get-prop name)
  (let* ([lst (get-prop comp)]
         [p (find-port port lst)])
    (if p
        (if (infinite-port? p)
            (void)
            (set-prop! comp (remove p lst)))
        (error "Couldn't find" port "in" (component-name comp) name))))

(define (consume-in! comp port)
  (void)
  ;; (consume! comp port set-component-ins! component-ins 'input)
  )
(define (consume-out! comp port)
  (void)
  ;; (consume! comp port set-component-outs! component-outs 'outputs)
  )

(define (add-submod! comp name mod)
  (hash-set! (component-submods comp) name mod))
(define (get-submod! comp name)
  (hash-ref (component-submods comp) name))

(define (add-edge! comp src src-port tar tar-port width)
  (let ([src-name (hash-ref (component-splits comp) src src)]
        [tar-name (hash-ref (component-splits comp) tar tar)]
        [src-port-name (port-name src-port)]
        [tar-port-name (port-name tar-port)])
    (add-directed-edge!
     (component-graph comp)
     `(,src-name . ,src-port-name)
     `(,tar-name . ,tar-port-name)
     width)))

;; (define (add-in-hole! comp var-name u uport)
;;   (hash-set! (component-holes comp) var-name
;;              (s-hole var-name `(,u . ,uport) #t))
;;   (add-vertex! (component-graph comp) `(,u . ,uport)))

;; (define (add-out-hole! comp var-name u uport)
;;   (hash-set! (component-holes comp) var-name
;;              (s-hole var-name `(,u . ,uport) #f))
;;   (add-vertex! (component-graph comp) `(,u . ,uport)))

(define (add-constraint! comp constr)
  (set-component-constraints! comp (cons constr (component-constraints comp))))

(define (connect! comp src src-portname tar tar-portname)
  (let* ([src-submod (get-submod! comp src)]
         [tar-submod (get-submod! comp tar)]
         [src-port (name->port src-portname (component-outs src-submod))]
         [tar-port (name->port tar-portname (component-ins tar-submod))])
    (if (= (port-width src-port) (port-width tar-port))
        (begin
          (consume-out! src-submod src-port)
          (consume-in! tar-submod tar-port)
          (add-edge! comp src src-port tar tar-port (port-width src-port)))
        (error "Port widths don't match!"
               src-port '!= tar-port))))

(define (split! comp name split-pt name1 name2)
  (define (port-eq x y) (equal? (port-name x) (port-name y)))
  (define (help port make-comp)
    (split-port-ok? port split-pt)
    (hash-set! (component-submods comp) name1 (make-comp split-pt))
    (hash-set! (component-submods comp)
               name2
               (make-comp (- (port-width port) split-pt))))
  (cond [(name->port name (component-ins comp))
         => (lambda (p)
              (help p input-component)
              (hash-set! (component-splits comp) name1 name)
              (hash-set! (component-splits comp) name2 name))]
        [(name->port name (component-outs comp))
         => (lambda (p)
              (help p output-component)
              (hash-set! (component-splits comp) name1 name)
              (hash-set! (component-splits comp) name2 name))]
        [else (error "Port not found in the inputs!")]))

;; (define (backtrack comp outlst)
;;   (apply append (map (lambda (vert)
;;                        (match vert
;;                          [(cons v _)
;;                           (map (lambda (kw) `(,v . ,kw))
;;                                (map port-name (component-ins (get-submod! comp v))))]))
;;                      outlst)))

;; (define (relevant-constraints comp var)
;;   (filter (lambda (con) (equal? (get-left con) var)) (component-constraints comp)))

;; (define (follow-back comp pair)
;;   (define cands (filter right-handed? (hash-values (component-holes comp))))
;;   (map s-hole-name (filter (lambda (c) (equal? (s-hole-pair c) pair)) cands)))

;; (define (get-neighs comp vertex)
;;   (define valid-ports (map port-name (component-ins (get-submod! comp vertex))))
;;   (define neighs (map (lambda (p)
;;                         (sequence->list (in-neighbors
;;                                          (transpose (component-graph comp))
;;                                          `(,vertex . ,p))))
;;                       valid-ports))
;;   (define traces (flatten (map (lambda (x) (follow-back comp x))
;;                                (map (lambda (x) `(,vertex . ,x)) valid-ports))))
;;   (define trace-deps (flatten (map get-dependencies
;;                                    (flatten
;;                                     (map (lambda (x) (relevant-constraints comp x))
;;                                          traces)))))
;;   (remove-duplicates (filter (lambda (p) (not (empty? (cdr p))))
;;                              (map (lambda (p n) `(,p . ,n)) valid-ports neighs))))

;; (define (follow-holes comp var)
;;   (define valid-ports (map port-name (component-ins (get-submod! comp var))))
;;   (define traces (flatten (map (lambda (x) (follow-back comp x))
;;                                (map (lambda (x) `(,var . ,x)) valid-ports))))
;;   (define trace-deps (map get-dependencies
;;                           (flatten
;;                            (map (lambda (x) (relevant-constraints comp x))
;;                                 traces))))
;;   (println (~v valid-ports trace-deps))
;;   (if (empty? trace-deps)
;;       '()
;;       (map (lambda (prt dep)
;;              (let ([con (match dep
;;                           [(cond-computation x y)
;;                            (cond-computation
;;                             (car (s-hole-pair (hash-ref (component-holes comp) x)))
;;                             (car (s-hole-pair (hash-ref (component-holes comp) y))))]
;;                           [(equal-computation x)
;;                            (equal-computation
;;                             (car (s-hole-pair (hash-ref (component-holes comp) x))))])])
;;                `(,prt . ,con)))
;;            valid-ports
;;            trace-deps)))

;; (define (stabilize comp inputs vertex)
;;   (if (member vertex (map port-name (component-ins comp)))
;;       (hash-ref inputs vertex)
;;       (let* ([neighs (get-neighs comp vertex)]
;;              [sub (get-submod! comp vertex)]
;;              [holes (map (lambda (x)
;;                            (match x
;;                              [(cons name (cond-computation x con))
;;                               (begin
;;                                 (println (~v 'there x))
;;                                 (if (= 1 (stabilize comp inputs con))
;;                                     `(,name . ,(stabilize comp inputs x))
;;                                     `(,name . undefined)))]
;;                              [(cons name (equal-computation x))
;;                               (begin
;;                                 (println (~v 'bye x))
;;                                 `(,name . ,(stabilize comp inputs x)))]))
;;                          (follow-holes comp vertex))]
;;              [vals (map (lambda (pair)
;;                           (match pair
;;                             [(cons name (cons (cons v _) _))
;;                              `(,name . ,(stabilize comp inputs v))]))
;;                         neighs)]
;;              [proc (component-proc sub)])
;;         (proc (make-hash (append holes vals))))))

;; (define (compute comp inputs)
;;   (if (component-primitive comp)
;;       ((component-proc comp) inputs)
;;       (flatten (map (lambda (v) (stabilize comp inputs v))
;;                     (map port-name (component-outs comp))))))
(define (top-order g)
  (define (check against lst)
    (if (foldl (lambda (x acc)
                 (or acc (member x against)))
               #f
               lst)
        #t
        #f))
  (define trans-g (transpose g))
  (reverse
   (foldl (lambda (x acc)
            (if (check (flatten acc) (sequence->list (in-neighbors trans-g x)))
                (cons `(,x) acc)
                (match acc
                  [(cons h tl) (cons (cons x h) tl)])))
          '(())
          (tsort g))))

;; (top-order (convert-graph (triv)))

;; (hash, comp-name) -> transformed hash
(define (transform comp inputs name)
  (define sub (get-submod! comp name))
  (define ins (map port-name (component-ins sub))) ; XXX: deal with port widths
  (make-immutable-hash
   (map (lambda (in)
          (define neighs (sequence->list (in-neighbors (transpose (component-graph comp)) `(,name . ,in))))
          `((,name . ,in) . ,(hash-ref inputs (car neighs))))
        ins)))

(define (submod-compute comp inputs name)
  (define ins (make-immutable-hash (hash-map inputs (lambda (k v) `(,(cdr k) . ,v)))))
  (make-immutable-hash
   (hash-map ((component-proc (get-submod! comp name)) ins)
             (lambda (k v) `((,name . ,k) . ,v)))))

;; (submod-compute (triv) (transform (triv) (make-immutable-hash '(((add2 . out) . 60))) 'out) 'out)

(define (println-ret x) (println x) x)

(define (compute comp inputs)
  (define order (cdr (top-order (convert-graph comp)))) ; throw away first element because they are inputs and already computed
  (define filled
    (foldl (lambda (lst acc)
             (foldl (lambda (x acc)
                      (hash-union acc (submod-compute comp (transform comp acc x) x)))
                    acc
                    lst))
           inputs
           order))
  (map (lambda (x)
         `(,(car x) . ,(hash-ref filled x)))
       (map (lambda (x) `(,(port-name x) . inf#)) (component-outs comp))))
(define (input-hash lst)
  (make-immutable-hash (map (lambda (x) `((,(car x) . inf#) . ,(cdr x))) lst)))
(compute (triv) (input-hash '((a . 20) (b . 10) (c . 30))))

(define (convert-graph comp)
  (define g (component-graph comp))
  (define newg (empty-graph))
  (for-each (lambda (edge)
              (match edge
                [(cons (cons u _) (cons (cons v _) _))
                 (add-directed-edge! newg u v)]))
            (get-edges g))
  newg)

(define (plot comp)
  (plot-graph (show-board (component-name comp)) (convert-graph comp)))
