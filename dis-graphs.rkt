#lang racket/gui
(require graph)
(require racket/gui/base
         mrlib/graph)
(provide show-board
         plot-graph)

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
    (init-field value radius)
    (super-new)
    (send this set-snipclass node-snip-class)

    (define/public (get-value) value)

    (define/override (get-extent dc x y width height descent space lspace rspace)
      (when width (set-box! width (* radius 2)))
      (when height (set-box! height (* radius 2)))
      (when descent (set-box! descent 0.0))
      (when space (set-box! space 0.0))
      (when lspace (set-box! lspace 0.0))
      (when rspace (set-box! rspace 0.0)))

    (define/override (draw dc x y . other)
      (send dc set-font (send the-font-list find-or-create-font 10 'default 'normal 'normal))
      (send dc set-text-background "black")
      ;; (send dc draw-ellipse (+ x 0) (+ y 0) (* 3 radius) (* 3 radius))
      (send dc draw-rectangle (+ x 0) (+ y 0) (* 2 radius) (* 2 radius))
      (define label (~a value))
      (send dc draw-text label (+ x (/ radius 2)) (+ y (/ radius 2))))))

(define node%
  (graph-snip-mixin node-snip%))

(define (plot-graph board g)
  ;; clear old graph
  (send board erase)

  ;; get all the vertices in the graph
  (define nodes
    (map (λ (v)
           (new node% [value v] [radius 10]))
         (get-vertices g)))

  ;; insert all the vertices into the board
  (for-each (λ (node) (send board insert node 100 100)) nodes)

  ;; add all the edges
  (map (λ (parent)
         (for-each (λ (neigh-l)
                (define obj-i
                  (index-where
                   nodes
                   (λ (item) (equal? neigh-l (send item get-value)))))
                (define child (list-ref nodes obj-i))
                (add-links parent child)
                (let* ([u (send parent get-value)]
                       [v (send child get-value)]
                       [label (if (= (edge-weight g u v) 1) "" (~a (edge-weight g u v)))])
                  (cond
                    [(has-edge? g u v)
                     (set-link-label parent child label)])))
              (get-neighbors g (send parent get-value))))
       nodes)

  ;; position the nodes on the board
  (dot-positioning board "dot"))

;; ==========================

(define (show-board name)
  (define board (new graph-board%))

   (define toplevel (new frame%
                         [label (~a name)]
                         [width (* 50 8)]
                         [height (* 50 8)]))

   (define canvas (new editor-canvas%
                       [parent toplevel]
                       [style '(no-hscroll no-vscroll)]
                       [horizontal-inset 0]
                       [vertical-inset 0]
                       [editor board]))

   (send toplevel show #t)
   (values board))


;; ==========================

;; (define g (directed-graph '((a b) (b c)) '(1 1)))
;; (define g (matrix-graph [[0 3 8 #f -4]
;;                          [#f 0 #f 1 7]
;;                          [#f 4 0 #f #f]
;;                          [2 1 -5 0 #f]
;;                          [#f #f #f 6 0]]))

;; (plot-graph (show-board) g)

;; (dot-positioning board "dot")
