#lang racket/base

(require "futil-prims.rkt"
         "port.rkt"
         "util.rkt"
         "futil-syntax-2.rkt")

(require macro-debugger/stepper)
(require futil/visualizer)

(define/component and3way
  ([a 32] [b 32] [c 32])
  ([out 32])
  ([new en (const 1)]
   [-> (@ en out) (@ this out)])
  (ifen (@ this a)
        (ifen (@ this b)
              (ifen (@ this c)
                    (enable en out)
                    (empty))
              (empty))
        (empty)))

;; (require macro-debugger/stepper)
(define/component comp/iterator
  ([start 32] [incr 32] [end 32] [en 32])
  ([out 32] [stop 32])
  ([new incr-reg comp/reg]
   [new end-reg comp/reg]
   [new add comp/add]
   [new cmp comp/trunc-sub]

   [new ins-and and3way]
   [-> (@ this start) (@ ins-and a)]
   [-> (@ this incr) (@ ins-and b)]
   [-> (@ this end) (@ ins-and c)]

   [-> (@ this incr) (@ incr-reg in)]
   [-> (@ this end) (@ end-reg in)]

   [new val-reg comp/res-reg]
   [new res-vel (const 1)]
   [-> (@ res-val out) (@ val-reg res)]

   [new add0 (const 0)]
   [-> (@ add0 out) (@ add right)]
   [-> (@ this start) (@ add left)]
   [-> (@ incr-reg out) (@ add right)]
   [-> (@ add out) (@ val-reg in)]
   [-> (@ val-reg out) (@ add left)]
   [-> (@ add out) (@ this out)]
   [-> (@ end-reg out) (@ cmp left)]
   [-> (@ add out) (@ cmp right)]
   [-> (@ cmp out) (@ this stop)])
  (seq
   [enable start incr end ins-and]
   [ifen (@ this en)
         (ifen (@ ins-and out)
               (seq
                [enable res-val val-reg]
                [enable start incr end incr-reg end-reg]
                [disable incr incr-reg end res-val])
               (disable add-zero start incr end res-val))
         (disable start incr incr-reg end res-val)]))

(plot-compute comp/iterator '((start . 0) (incr . 1) (end . 10) (en . 1)))

;; (expand/step
;;  #'(define/namespace stdlib
;;      (define (const n)
;;        (default-component
;;          'const
;;          '()
;;          (list (port 'out 32))
;;          (keyword-lambda () () [out => n])))

;;      (define (comp/reg)
;;        (default-component
;;          'reg
;;          (list (port 'in 32))
;;          (list (port 'out 32))
;;          (keyword-lambda (mem-val# in) ()
;;                          [out => (if in
;;                                      (blocked in mem-val#)
;;                                      (if mem-val#
;;                                          mem-val#
;;                                          (blocked #f #f)))])
;;          #:memory-proc (lambda (old st)
;;                          (define new-v (dict-ref st 'in))
;;                          (if new-v new-v old))))

     ;; (define/component and3way
     ;;   ([a 32] [b 32] [c 32])
     ;;   ([out 32])
     ;;   ([new en (const 1)]
     ;;    [-> (@ en out) (@ this out)])
     ;;   (ifen (@ this a)
     ;;         (ifen (@ this b)
     ;;               (ifen (@ this c)
     ;;                     (enable en out)
     ;;                     (empty))
     ;;               (empty))
     ;;         (empty)))

     ;; ))

