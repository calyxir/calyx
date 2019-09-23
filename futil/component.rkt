#lang racket/base

(require graph
         threading
         racket/hash
         racket/dict
         racket/list
         racket/match
         racket/pretty
         "port.rkt"
         "util.rkt")

(provide (struct-out component)
         (struct-out blocked)
         (struct-out connection)
         (struct-out decl)
         empty-graph
         transform-control
         input-component
         output-component
         default-component
         make-constant
         connect!
         add-submod!
         get-submod!
         split!
         commit-transpose!
         convert-graph)

(struct connection (src dest) #:transparent)
(struct decl (var comp) #:transparent)

(struct component (;; name of the component
                   name
                   ;; list of input ports
                   ins
                   ;; list of output ports
                   outs
                   ;; hashtbl of sub components keyed on their name
                   submods
                   ;; hashtbl keeping track of split nodes
                   splits
                   ;; ast
                   control
                   ;; procedure representing this modules computation
                   [proc #:mutable]
                   ;; procedure for setting memory
                   memory-proc
                   ;; graph representing internal connections
                   graph
                   ;; inverse of graph
                   [transpose #:mutable]))

;; structure for blocked values
(struct blocked (dirty clean) #:transparent)

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
                             #f
                             (keyword-lambda (inf#) () [inf# => inf#])
                             (lambda (old st) #f)
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
                              #f
                              (keyword-lambda (inf#) () [inf# => inf#])
                              (lambda (old st) #f)
                              (empty-graph)
                              #f))

(define (transform-control control) control)

;; given a name, list of input ports, and list of output ports, creates
;; a component an empty graph and the appropriate input and output ports
;; in the hashtable.
(define (default-component
          name
          ins
          outs
          proc
          #:control [control #f]
          #:memory-proc [memory-proc
                         (lambda (old st) #f)])
  (let ([htbl (make-hash)]
        [g (empty-graph)])
    (for-each (lambda (p) ; p is a port
                (dict-set! htbl (port-name p) (input-component (port-width p)))
                (add-vertex! g `(,(port-name p) . inf#)))
              ins)
    (for-each (lambda (p)
                (dict-set! htbl (port-name p) (output-component (port-width p)))
                (add-vertex! g `(,(port-name p) . inf#)))
              outs)
    (component
     name
     ins
     outs
     htbl          ; sub-mods
     (make-hash)   ; splits
     control
     proc
     memory-proc
     g
     #f)))

(define (make-constant n width)
  (default-component
    n
    '()
    (list (port 'inf# width))
    (keyword-lambda () () [inf# => n])))

(define (add-submod! comp name mod)
  (dict-set! (component-submods comp) name mod))
(define (get-submod! comp name)
  (dict-ref (component-submods comp) name))

(define (add-edge! comp src src-port tar tar-port width)
  (let ([src-name (dict-ref (component-splits comp) src src)]
        [tar-name (dict-ref (component-splits comp) tar tar)]
        [src-port-name (port-name src-port)]
        [tar-port-name (port-name tar-port)])
    (add-directed-edge!
     (component-graph comp)
     `(,src-name . ,src-port-name)
     `(,tar-name . ,tar-port-name)
     width)))

(define (connect! comp src src-portname tar tar-portname)
  (let* ([src-submod (get-submod! comp src)]
         [tar-submod (get-submod! comp tar)]
         [src-port (name->port (component-name comp) src-portname (component-outs src-submod))]
         [tar-port (name->port (component-name comp) tar-portname (component-ins tar-submod))])
    (if (= (port-width src-port) (port-width tar-port))
        (add-edge! comp src src-port tar tar-port (port-width src-port))
        (error "Port widths don't match!"
               src-port '!= tar-port))))

(define (split! comp name split-pt name1 name2)
  (define (port-eq x y) (equal? (port-name x) (port-name y)))
  (define (help port make-comp)
    (split-port-ok? port split-pt)
    (dict-set! (component-submods comp) name1 (make-comp split-pt))
    (dict-set! (component-submods comp)
               name2
               (make-comp (- (port-width port) split-pt))))
  (cond [(name->port (component-name comp) name (component-ins comp))
         => (lambda (p)
              (help p input-component)
              (dict-set! (component-splits comp) name1 name)
              (dict-set! (component-splits comp) name2 name))]
        [(name->port (component-name comp) name (component-outs comp))
         => (lambda (p)
              (help p output-component)
              (dict-set! (component-splits comp) name1 name)
              (dict-set! (component-splits comp) name2 name))]
        [else (error "Port not found in the inputs!")]))

(define (commit-transpose! c)
  (set-component-transpose! c (transpose (component-graph c))))

(define (convert-graph comp [vals #f] #:transpose [transpose #f])
  (define g (if transpose (component-transpose comp) (component-graph comp)))
  (define newg (empty-graph))
  (~> (get-vertices g)
      (map car _)
      remove-duplicates
      (for-each (lambda (v) (add-vertex! newg v)) _))
  (for-each
    (lambda (edge)
      (match edge
        [(cons (cons u _) (cons (cons v _) _))
         (if vals
           (add-directed-edge! newg u v (dict-ref vals (car edge)))
           (add-directed-edge! newg u v))]))
    (get-edges g))
  newg)
