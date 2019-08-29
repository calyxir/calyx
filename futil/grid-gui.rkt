#lang racket/gui

(require racket/gui/base
         graph
         mrlib/graph
         racket/set
         racket/hash
         "component.rkt"
         "port.rkt"
         "futil-prims.rkt")

(define (round-up-to n to)
  (+ n
     (-
      to
      (modulo n to))))

(define (round-to-closest n to)
  (let ([n (round n)]
        [to (round to)])
    (if (< (modulo n to) (/ to 2))
        (- n
           (modulo n to))
        (+ n
           (- to
              (modulo n to))))))

(define node-snip-class
  (make-object
   (class snip-class%
     (super-new)
     (send this set-classname "node-snip"))))

(send (get-the-snip-class-list) add node-snip-class)

(define node-snip%
  (class snip%
    (init-field value
                active
                grid-width
                inputs
                outputs)
    (super-new)
    (send this set-snipclass node-snip-class)

    (define num-inputs (length inputs))
    (define num-outputs (length outputs))

    (define char-width 12)
    (define value-width
      (round-up-to
       (+ 4
          (* char-width (string-length (~a value))))
       grid-width))

    (define value-height
      (round-up-to
       (* grid-width (+ (max num-inputs num-outputs) 0))
       grid-width))

    (define/public (get-width) value-width)
    (define/public (get-height) value-height)

    (define/override (get-extent dc x y width height descent space lspace rspace)
      (when width (set-box! width value-width))
      (when height (set-box! height value-height))
      (when descent (set-box! descent 0.0))
      (when space (set-box! space 0.0))
      (when lspace (set-box! lspace 0.0))
      (when rspace (set-box! rspace 0.0)))

    (define/override (draw dc x y . other)
      (define font
        (send the-font-list find-or-create-font 10 'modern 'normal 'normal))
      (send dc set-font font)
      (define label (~a value))
      (cond
        [(not active)
         (send* dc
           (set-pen "blue" 1 'solid)
           (set-brush "blue" 'bdiagonal-hatch)
           (draw-rectangle x y value-width 20)
           (set-text-foreground "red")
           (draw-text label
                      (+ x (/ value-width 4))
                      (+ y (/ value-height 4))))]
        [active
         (send* dc
           (set-text-background "black")
           (draw-text label
                      (+ x (/ value-width 4))
                      (+ y (/ value-height 4)))
           (set-pen "black" 1 'solid)
           (set-brush "white" 'solid)
           (draw-rectangle (+ 2 x) (+ 2 y) (- value-width 4) (- value-height 4))
           (set-text-foreground "black")
           (draw-text label
                      (+ x (/ value-width 4))
                      (+ y (/ value-height 4))))])
      (for ([in (in-range num-inputs)])
        (define in-y (+ y grid-width (* in grid-width) -2))
        (send* dc
          (set-pen "blue" 1 'solid)
          (set-brush "blue" 'solid)
          (draw-ellipse (- x 0) (- in-y 2) 4 4)))
      (for ([out (in-range num-outputs)])
        (define out-y (+ y grid-width (* out grid-width) -2))
        (send* dc
          (set-pen "blue" 1 'solid)
          (set-brush "blue" 'solid)
          (draw-ellipse (+ x value-width -4) out-y 4 4))))

    (define/public (get-in-port-pos name x y)
      (values
       (round (/ x grid-width))
       (+ (round (/ y grid-width))
          (index-of (map port-name inputs) name)
          1)))

    (define/public (get-out-port-pos name x y)
      (values
       (round (/ (+ x value-width) grid-width))
       (+ (round (/ y grid-width))
          (index-of (map port-name outputs) name)
          1)))
    ))

(define node%
  (graph-snip-mixin node-snip%))

(define graph-board%
  (class pasteboard%
    (inherit get-admin)
    (init-field [(comp component)] grid-width)
    (super-new)

    (define grid-cache #f)
    (define hover-list '())

    ;; initialize nodes
    (send this begin-edit-sequence)
    (define nodes
      (make-immutable-hash
       (map (lambda (vert)
              (let* ([sub (get-submod! comp vert)]
                     [node (new node%
                                [inputs (component-ins sub)]
                                [outputs (component-outs sub)]
                                [value vert]
                                [active #t]
                                [grid-width grid-width])])
                (send this insert node 0 0)
                `(,vert . ,node)))
            (get-vertices (convert-graph comp)))))
    (layout)
    (send this end-edit-sequence)

    (define/override (on-paint before? dc topx topy width height . other)
      (when before?
        (draw-grid dc width height)
        (draw-wires dc))
      (super on-paint before? dc topx topy width height . other))

    (define/augment (on-display-size)
      (set! grid-cache #f))

    (define/private (empty-cache)
      (define admin (get-admin))
      (if admin
          (let ([xb (box 0)]
                [yb (box 0)]
                [wb (box 0)]
                [hb (box 0)])
            (send admin get-max-view xb yb wb hb)
            (make-bitmap (unbox wb) (unbox hb)))
          #f)
      )

    (define/public (draw-grid dc width height)
      (unless grid-cache
        (set! grid-cache (empty-cache))
        (define grid-dc (make-object bitmap-dc% grid-cache))
        (for ([x (in-range grid-width width grid-width)])
          (for ([y (in-range grid-width height grid-width)])
            (send* grid-dc
              (set-pen "gray" 1 'solid)
              (set-brush "gray" 'solid)
              (draw-ellipse (- x 1) (- y 1) 2 2))))
        (send grid-dc set-bitmap #f))

      (send dc draw-bitmap grid-cache 0 0))

    (define/public (draw-wires dc)
      (map (lambda (x)
             (draw-path dc (car x) (cadr x)))
           (get-edges (component-graph comp))))

    (define/augment (after-move-to snip x y dragging?)
      (when (not dragging?)
        (send this begin-edit-sequence)
        (send this move-to snip
              (round-to-closest x grid-width)
              (round-to-closest y grid-width))
        (send this end-edit-sequence)))

    (define/override (on-event evt)
      (cond
        [(send evt leaving?)
         (set! hover-list '())]
        [(or (send evt entering?)
             (send evt moving?))
         (let ([ex (send evt get-x)]
               [ey (send evt get-y)])
           (set! hover-list (get-nodes-at ex ey)))]
        [else (void)])
      (super on-event evt)
      )

    (define/private (get-location node)
      (let ([x (box 0)]
            [y (box 0)]
            [w (send node get-width)]
            [h (send node get-height)])
        (send this get-snip-location node x y)
        (values (unbox x)
                (unbox y)
                w h)))

    (define/private (in-rectangle? x y p1x p1y p2x p2y)
      (and (<= (min p1x p2x) x (max p1x p2x))
           (<= (min p1y p2y) y (max p1y p2y))))

    (define/private (get-nodes-at ex ey)
      (filter-map
       (lambda (pair)
         (define-values (key node) (values (car pair) (cdr pair)))
         (let-values ([(x y w h) (get-location node)])
           (if (in-rectangle? ex ey x y
                              (+ x w) (+ y h))
               key
               #f
               )))
       (hash->list nodes))
      )

    (define/private (layout)
      (dot-positioning this "dot"))

    (define/private (draw-path-on-grid dc vpoints hover)
      (define realpoints
        (map (lambda (x)
               (cons
                (* grid-width (car x))
                (* grid-width (cdr x))))
             vpoints))
      (define real-xs
        (map (lambda (p)
               (car p))
             realpoints))
      (define real-ys
        (map (lambda (p)
               (cdr p))
             realpoints))
      (if hover
          (send dc set-pen "blue" 2 'solid)
          (send dc set-pen "green" 2 'solid))
      (send dc draw-lines realpoints)
      (send this refresh
            (apply min real-xs) (apply min real-ys)
            (apply max real-xs) (apply max real-ys)
            'no-caret
            #f))

    (define/private (draw-path dc start end)
      (define start-node (hash-ref nodes (car start)))
      (define-values (start-vx start-vy)
        (let-values ([(x y w h) (get-location start-node)])
          (send start-node get-out-port-pos
                (cdr start) x y)))

      (define end-node (hash-ref nodes (car end)))
      (define-values (end-vx end-vy)
        (let-values ([(x y w h) (get-location end-node)])
          (send end-node get-in-port-pos
                (cdr end) x y)))

      (define path
        `((,start-vx . ,start-vy)
          (,(add1 start-vx) . ,start-vy)
          (,(sub1 end-vx) . ,end-vy)
          (,end-vx . ,end-vy)))

      (define hover
        (or (member (car start) hover-list)
            (member (car end) hover-list )))

      (draw-path-on-grid dc path hover))
    ))

(define (show comp #:grid-width [grid-width 20])
  (define toplevel
    (new frame%
         [label "Grid"]
         [width (* 50 11)]
         [height (* 50 10)]))

  (define board (new graph-board%
                     [component comp]
                     [grid-width grid-width]))

  (define canvas
    (new editor-canvas%
         [parent toplevel]
         [style '(auto-vscroll auto-hscroll)]
         [horizontal-inset 0]
         [vertical-inset 0]
         [editor board]))

  (send toplevel show #t))

;; (define g
;;   (matrix-graph [[0 3 8 #f -4]
;;                  [#f 0 #f 1 7]
;;                  [#f 4 0 #f #f]
;;                  [2 #f -5 0 #f]
;;                  [#f #f #f 6 0]]))
;; (get-neighbors g 0)
(require futil)

(define/module test ((a : 32) (b : 32)) ((out : 32))
  ([add = new comp/add]
   [a -> add @ left]
   [b -> add @ right]
   [add @ out -> out])
  [])
(show (test))
