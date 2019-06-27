#lang racket
(require graph
         racket/hash
         "port.rkt")
(provide keyword-lambda
         (struct-out constr)
         (struct-out control-pair)
         (struct-out component)
         transform-control
         input-component
         output-component
         default-component
         make-constant
         connect!
         add-submod!
         get-submod!
         add-control!
         ;; add-constraint!
         split!
         top-order
         compute-step
         compute
         input-hash
         convert-graph)

(define-syntax-rule (keyword-lambda (arg ...)
                                    ([var = body1 ...] ...)
                                    [kw => body2 ...] ...)
  (lambda (h)
    (define arg (hash-ref h 'arg)) ...
    (define var (begin body1 ...)) ...
    (make-hash `((kw . ,(begin body2 ...)) ...))))

(struct constr (condition tbranch fbranch) #:transparent)
(struct control-pair (inactive constr) #:transparent)

(struct component (name                       ;; name of the component
                   [ins #:mutable]            ;; list of input ports
                   [outs #:mutable]           ;; list of output ports
                   submods                    ;; hashtbl of sub components keyed on their name
                   splits                     ;; hashtbl keeping track of split nodes
                   ;; control                   ;; a hashtbl from names to sets of names representing control points
                   [control #:mutable]        ;; list of (inactive lst * constr list) tuples XXX: remove mut
                   [proc #:mutable]           ;; procedure representing this modules computation XXX: remove mut
                   graph                      ;; graph representing internal connections
                   ;; primitive                  ;; true when this component is primitive
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
                             (list (control-pair '() '()))
                             (keyword-lambda (inf#) () [inf# => inf#])
                             (empty-graph)))

;; creates a component with a single infinite input port of width w
;; and no output ports. Designed to be used as the output of a component.
(define (output-component w) (component
                              'output
                              (list (port 'inf# w))
                              '()
                              (make-hash) ; submods
                              (make-hash) ; splits
                              (list (control-pair '() '()))
                              (keyword-lambda (inf#) () [inf# => inf#])
                              (empty-graph)))

(define (list-subtraction l1 l2)
  (foldl (lambda (x acc)
           (remove x acc))
         l1
         l2))

(define (transform-control control)
  ;; (define control (component-control (add4)))
  (define all-inactive (map control-pair-inactive control))
  (define all-constr (map control-pair-constr control))
  (define grid (make-list (length control) (flatten all-inactive)))
  (define edited
    (map (lambda (g a)
           (list-subtraction g a))
         grid
         all-inactive))
  (map (lambda (e a) (control-pair e a))
       edited
       all-constr))

;; TODO: maybe add vertices for ins and outs

;; given a name, list of input ports, and list of output ports, creates
;; a component an empty graph and the appropriate input and output ports
;; in the hashtable.
(define (default-component name ins outs proc [control (list (control-pair '() '()))])
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
     control
     proc
     g)))

(define (make-constant n width)
  (default-component n '() (list (port 'inf# width)) (keyword-lambda () () [inf# => n])))

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

;; (define (add-constraint! comp constr)
;;   (set-component-constraints! comp (cons constr (component-constraints comp))))

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


(define (add-control! comp name names)
  (void)
  ;; (define vals (flatten (hash-values (component-control comp))))
  ;; (for-each (lambda (n) (if (hash-has-key? (component-submods comp) n)
  ;;                           (void)
  ;;                           (error n "not a sub-module"))) names)
  ;; (if (ormap (lambda (n) (member n vals)) names)
  ;;     (error "One of" name "was already used in another control point")
  ;;     (hash-set! (component-control comp) name names))
  )

(define (distMatrix comp)
  (define copy (graph-copy (convert-graph comp)))
  (for-each (lambda (x)
              (add-directed-edge! copy 'start# x))
            (map port-name (component-ins comp)))
  (let-values ([(distMat _) (bfs copy 'start#)])
    (hash-remove (make-immutable-hash (hash-map distMat (lambda (k v) `(,k . ,(- v 1)))))
                 'start#)))

(define (top-order comp)
  (define sorted (sort (hash->list (distMatrix comp))
                       (lambda (x y)
                         (< (cdr x) (cdr y)))))
  (reverse
   (car (foldl (lambda (x acc)
                 (if (= (cdr x) (cdr acc))
                     `(,(cons (cons (car x) (caar acc)) (cdar acc)) . ,(cdr acc))
                     `(,(cons `(,(car x)) (car acc)) . ,(+ 1 (cdr acc)))))
               '(() . -1)
               sorted))))

(define (transform comp inputs name)
  ;; (println (~v 'transform name (component-ins comp)))
  (if (findf (lambda (x) (equal? name (port-name x))) (component-ins comp))
      (make-immutable-hash `(((,name . inf#) . ,(hash-ref inputs `(,name . inf#)))))
      (begin
        (let* ([sub (get-submod! comp name)]
               [ins (map port-name (component-ins sub))])  ; XXX: deal with port widths
          ;; (println (~v 'transform name ': inputs '-> ins))
          (make-immutable-hash
           (map (lambda (in)
                  (define neighs
                    (sequence->list (in-neighbors (transpose (component-graph comp)) `(,name . ,in))))
                  (define filt-neighs-vals (filter-map (lambda (x) (hash-ref inputs x)) neighs))
                  (define neighs-vals
                    (if (empty? filt-neighs-vals)
                        (map (lambda (x) (hash-ref inputs x)) neighs)
                        filt-neighs-vals))
                  ;; (println (~v 'neighs neighs neighs-vals))
                  `((,name . ,in) . ,(car neighs-vals)))
                ins))))))

(define (mint-inactive-hash comp name)
  (make-immutable-hash (map
                        (lambda (x)
                          `((,name . ,(port-name x)) . #f))
                        (append
                         (component-outs (get-submod! comp name))
                         (filter-map
                          (lambda (x) (and (equal? name (port-name x)) (port 'inf# (port-width x))))
                          (component-outs comp))))))

(define (submod-compute comp inputs name)
  (define ins (make-immutable-hash (hash-map inputs (lambda (k v) `(,(cdr k) . ,v)))))
  ;; (println (~v 'submod inputs '-> ins))
  (if (andmap (lambda (x) x) (hash-values ins))
      (make-immutable-hash
       (hash-map ((component-proc (get-submod! comp name)) ins)
                 (lambda (k v) `((,name . ,k) . ,v))))
      (begin
        ;; (println (~v 'mint name (mint-inactive-hash comp name)))
        (mint-inactive-hash comp name))
      ))

(define (c-hash-union h1 h2)
  (hash-union h1 h2 #:combine (lambda (v1 v2)
                                (cond
                                  [(not v1) v2]
                                  [(not v2) v1]
                                  [else v2]))))

(define (compute-step comp inputs [inactive-lst '()] [constrs '()])
  (define order (top-order comp))
  (define inactive (remove-duplicates
                    (flatten
                     (foldl (lambda (c acc)
                              (append acc
                                      (if (equal? 0 (hash-ref inputs (constr-condition c)))
                                          (constr-tbranch c)
                                          (constr-fbranch c))))
                            inactive-lst
                            constrs))))
  ;; (println inactive)
  (define filled
    (foldl (lambda (lst acc)
             (foldl (lambda (x acc)
                      ;; (println (~v x (member x inactive)))
                      ;; (println (~v acc))
                      (if (member x inactive)
                          ; inactive
                          (c-hash-union acc (mint-inactive-hash comp x))
                          ; active
                          (c-hash-union acc (submod-compute comp (transform comp acc x) x))))
                    acc
                    lst))
           inputs
           order))
  (values
   filled
   (map (lambda (x)
          `(,(car x) . ,(hash-ref filled x)))
        (map (lambda (x) `(,(port-name x) . inf#)) (component-outs comp)))))

(define (compute comp lst)
  (define inputs (input-hash comp lst))
  (define res
    (foldl (lambda (p acc)
             ;; (println acc)
             (match p
               [(control-pair inactive constrs)
                (let-values ([(state vals) (compute-step comp (caar acc) inactive constrs)])
                  ;; (println (~v 'compute state vals))
                  (cons (cons state (car acc)) (cons vals (cdr acc))))]))
           (cons (list inputs) '())
           (component-control comp)))
  (match res
    [(cons states vals)
     (cons (reverse states) (reverse vals))]))

(define (input-hash comp lst)
  (define empty-hash
    (make-immutable-hash
     (map (lambda (x) `(,x . #f))
          (append
           (map car (get-edges (component-graph comp)))
           (map (lambda (p)
                  `(,(port-name p) . inf#))
                (component-outs comp))))))
  (c-hash-union empty-hash
                (make-immutable-hash (map (lambda (x) `((,(car x) . inf#) . ,(cdr x))) lst))))

(define (convert-graph comp [vals #f])
  (define g (component-graph comp))
  (define newg (empty-graph))
  (for-each (lambda (edge)
              (match edge
                [(cons (cons u _) (cons (cons v _) _))
                 (if vals
                     (begin
                       (add-directed-edge! newg u v (hash-ref vals (car edge))))
                     (add-directed-edge! newg u v)
                     )
                 ]))
            (get-edges g))
  newg)


;; (define (plot comp)
;;   (plot-graph (show-board (component-name comp)) (convert-graph comp)))
