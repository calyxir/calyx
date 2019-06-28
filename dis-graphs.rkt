#lang racket/gui
(require graph)
(require racket/gui/base
         mrlib/graph
         "component.rkt")
(provide show-board
         plot
         animate)

(define graph-board%
  (graph-pasteboard-mixin pasteboard%))

(define node-snip-class
  (make-object
   (class snip-class%
     (super-new)
     (send this set-classname "node-snip"))))

(send (get-the-snip-class-list) add node-snip-class)

(define node-snip%
  (class snip%
    (init-field value color)
    (super-new)
    (send this set-snipclass node-snip-class)

    (define char-width 12)
    (define value-width
      (+ 4
         (* char-width (string-length (~a value)))))

    (define/public (get-value) value)
    (define/public (get-width) value-width)

    (define/override (get-extent dc x y width height descent space lspace rspace)
      (when width (set-box! width value-width))
      (when height (set-box! height 20))
      (when descent (set-box! descent 0.0))
      (when space (set-box! space 0.0))
      (when lspace (set-box! lspace 0.0))
      (when rspace (set-box! rspace 0.0)))

    (define/override (draw dc x y . other)
      (define font (send the-font-list find-or-create-font 10 'modern 'normal 'normal))
      (send dc set-font font)
      (send dc set-text-background "black")
      (send dc set-pen color 1 'solid)
      (send dc draw-rectangle (+ x 0) (+ y 0) value-width 20)
      (define label (~a value))
      (send dc draw-text label (+ x (/ value-width 4)) (+ y 3)))))

(define node%
  (graph-snip-mixin node-snip%))

;; (define (top-order g)
;;   (define (check against lst)
;;     (if (foldl (lambda (x acc)
;;                  (or acc (member x against)))
;;                #f
;;                lst)
;;         #t
;;         #f))
;;   (define trans-g (transpose g))
;;   (reverse
;;    (foldl (lambda (x acc)
;;             (if (check (flatten acc) (sequence->list (in-neighbors trans-g x)))
;;                 (cons `(,x) acc)
;;                 (match acc
;;                   [(cons h tl) (cons (cons x h) tl)])))
;;           '(())
;;           (tsort g))))

(define (plot-comp board comp vals inactive)
  ;; clear old graph
  (send board erase)

  (define spacing 75)
  (define center 250)

  ;; insert all the vertices into the board
  (define nodes
    (flatten (map (lambda (vert-row j)
                    (map (lambda (vert i)
                           (let* ([color (if (member vert inactive) "red" "black")]
                                  [node (new node% [value vert] [color color])]
                                  [size (* spacing (- (length vert-row) 1))]
                                  [node-size (send node get-width)]
                                  [xoff (- center (/ size 2))])
                             (send board insert node
                                   (- (+ xoff (* spacing i)) node-size) (* spacing (+ j 1)))
                             node))
                         vert-row
                         (build-list (length vert-row) values)))
                  (top-order comp)
                  (build-list (length (top-order comp)) values))))

  (define g (convert-graph comp vals))

  ;; add all the edges
  (map (lambda (parent)
         (for-each (lambda (neigh-l)
                     (define obj-i
                       (index-where
                        nodes
                        (lambda (item) (equal? neigh-l (send item get-value)))))
                     (define child (list-ref nodes obj-i))
                     (add-links parent child)
                     (let* ([u (send parent get-value)]
                            [v (send child get-value)]
                            [label (~a (if (= +inf.0 (edge-weight g u v))
                                           ""
                                           (edge-weight g u v)))])
                       (cond
                         [(has-edge? g u v)
                          (set-link-label parent child label)])))
                   (get-neighbors g (send parent get-value))))
       nodes))

;; ==========================

(define (show-board name)
  (define board (new graph-board%))

  (define toplevel (new frame%
                        [label name]
                        [width (* 50 10)]
                        [height (* 50 10)]))

  (define canvas (new editor-canvas%
                      [parent toplevel]
                      [style '(no-hscroll no-vscroll)]
                      [horizontal-inset 0]
                      [vertical-inset 0]
                      [editor board]))

  (send toplevel show #t)
  board)

(define (plot comp [vals #f] [inactive '()] [suffix ""])
  (plot-comp (show-board (~a (component-name comp) "-" suffix)) comp vals inactive))

(define (animate comp inputs)
  (define hashs (rest (car (compute comp inputs))))
  (define control (map control-pair-inactive (component-control comp)))
  (map (lambda (h c i) (plot comp h c i)) hashs control (build-list (length hashs) values)))

;; ==========================

;; (define g (directed-graph '((a b) (b short)) '(1 1)))
;; (define board (show-board "test"))
;; (plot-graph board g)
;; (dot-positioning board "dot")

;; (define g (directed-graph '((a b) (b c)) '(1 1)))
;; (define g (matrix-graph [[0 3 8 #f -4]
;;                          [#f 0 #f 1 7]
;;                          [#f 4 0 #f #f]
;;                          [2 1 -5 0 #f]
;;                          [#f #f #f 6 0]]))

;; (plot-graph (show-board) g)

;; (dot-positioning board "dot")
