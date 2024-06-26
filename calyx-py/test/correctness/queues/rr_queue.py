# pylint: disable=import-error
import fifo
import calyx.builder as cb
import calyx.queue_call as qc

# This determines the maximum possible length of the queue:
# The max length of the queue will be 2^QUEUE_LEN_FACTOR.
QUEUE_LEN_FACTOR = 4

def invoke_subqueue(queue_cell, cmd, value, ans, err) -> cb.invoke:
   """Invokes the cell {queue_cell} with:
   {cmd} passed by value
   {value} passed by value
   {ans} passed by reference
   {err} passed by reference
   """
   return cb.invoke(
       queue_cell,
       in_cmd=cmd,
       in_value=value,
       ref_ans=ans,
       ref_err=err,
   )

def insert_pifo(
   prog,
   name,
   fifos,
   boundaries,
   n_flows, # the number of flows
   queue_len_factor=QUEUE_LEN_FACTOR,
   stats=None,
   static=False,
):
   """Inserts the component `pifo` into the program.
   """

   pifo: cb.ComponentBuilder = prog.component(name)
   cmd = pifo.input("cmd", 2) # the size in bits is 2
   # If this is 0, we pop. If it is 1, we peek.
   # If it is 2, we push `value` to the queue.
   value = pifo.input("value", 32)  # The value to push to the queue

   fifo_cells = []
   for num in range(len(fifos)):
       name = "queue_" + str(num)
       cell = pifo.cell(name, fifos[num])
       fifo_cells.append(cell)

   # If a stats component was provided, declare it as a cell of this component.
   if stats:
       stats = pifo.cell("stats", stats, is_ref=True)

   flow = pifo.reg(32, "flow")  # The flow to push to: 0 to n.
   # We will infer this using a separate component;
   # it is a function of the value being pushed.
  
   ans = pifo.reg(32, "ans", is_ref=True)
   # If the user wants to pop, we will write the popped value to `ans`.

   err = pifo.reg(1, "err", is_ref=True)
   # We'll raise this as a general error flag for overflow and underflow.

   length = pifo.reg(32, "length")  # The active length of the PIFO.

   # A register that marks the next sub-queue to `pop` from.
   hot = pifo.reg(32, "hot")
   og_hot = pifo.reg(32, "og_hot")
   copy_hot = pifo.reg_store(og_hot, hot.out) # og_hot := hot.out
   restore_hot = pifo.reg_store(hot, og_hot.out) # hot := og_hot.out

   max_queue_len = 2**queue_len_factor

   adder = pifo.add(32, "adder_reg")
   div_reg = pifo.div_pipe(32, "div_reg")
   i = pifo.reg(32, "i")

   # Some equality checks.
   hot_eq_n = pifo.eq_use(hot.out, n_flows-1, cellname="hot_eq_n") #bc 0-based indexing
   len_eq_0 = pifo.eq_use(length.out, 0, cellname="len_eq_0")
   len_eq_max_queue_len = pifo.eq_use(length.out, max_queue_len, cellname="len_eq_maxq")
   cmd_eq_0 = pifo.eq_use(cmd, 0, cellname="cmd_eq_0")
   cmd_eq_1 = pifo.eq_use(cmd, 1, cellname="cmd_eq_1")
   cmd_eq_2 = pifo.eq_use(cmd, 2, cellname="cmd_eq_2")
   err_eq_0 = pifo.eq_use(err.out, 0, cellname="err_eq_0")
   err_neq_0 = pifo.neq_use(err.out, 0, cellname="err_neq_0")

   incr_hot = pifo.incr(hot)
   raise_err = pifo.reg_store(err, 1, "raise_err")  # err := 1
   lower_err = pifo.reg_store(err, 0, "lower_err")  # err := 0
   reset_hot = pifo.reg_store(hot, 0, "reset_hot") # hot := 0

   len_incr = pifo.incr(length)  # len++
   len_decr = pifo.decr(length)  # len--

   handles = []
   for n in range(n_flows):
       handle = cb.if_with(pifo.eq_use(hot.out, cb.const(32, n)),
       invoke_subqueue(fifo_cells[n], cmd, value, ans, err))
       handles.append(handle)

   flow_handles = []
   for b in range(n_flows):
    #    _ , val_lt_cell = pifo.le_store_in_reg(value, boundaries[b+1])
    #    _ , val_gt_cell = pifo.ge_store_in_reg(value, boundaries[b])
    #    #val_add = pifo.add_store_in_reg(val_lt_cell.out, val_gt_cell.out, val_add_cell)
    #    _ , flow_reg = pifo.eq_store_in_reg(val_lt_cell.out, val_gt_cell.out)
       handle1 = cb.if_with(pifo.le_use(value, boundaries[b+1]), 
       cb.if_with(pifo.ge_use(value, boundaries[b]), invoke_subqueue(fifo_cells[b], cmd, value, ans, err)))

       flow_handles.append(handle1)

   # The main logic.
   pifo.control += cb.par(
       # Was it a pop, peek, or a push? We can do all cases in parallel.
       cb.if_with(
           # Did the user call pop?
           cmd_eq_0,
           cb.if_with( len_eq_0, raise_err,  # The queue is empty: underflow.
               [  # The queue is not empty. Proceed.
                   lower_err,
                   [ handles,
                       # Our next step depends on whether `fifos[hot]` raised the error flag.
                       cb.while_with( err_neq_0,
                           [  # `fifo_cells[hot]` raised an error.
                               # We'll try to pop from `fifo_cells[hot+1]`.
                               # We'll pass it a lowered err
                               lower_err,
                               cb.if_with(hot_eq_n, reset_hot, incr_hot),
                               handles, #TODO handles is the problem?!?!
                           ], # `queue[hot+n]` succeeded. Its answer is our answer.
                       ),                             
                   ],
                   cb.if_with(hot_eq_n, reset_hot, incr_hot), # are reset_hot anf incr_hot in the right order?
                   len_decr, 
               ],
           ),
       ),
       cb.if_with(
           # Did the user call peek?
           cmd_eq_1,
           cb.if_with( len_eq_0, raise_err,  # The queue is empty: underflow.
               [  # The queue is not empty. Proceed.
                   lower_err,
                   copy_hot,
                   [ handles,
                       cb.while_with(
                           err_neq_0,
                           [  # `fifo_cells[hot]` raised an error.
                               # We'll try to peek from `fifo_cells[hot+1]`.
                               # We'll pass it a lowered `err`.
                               lower_err,
                               cb.if_with(hot_eq_n, reset_hot, incr_hot), # increment hot and invoke_subqueue on the next one
                               handles
                           ],
                       ),
                       # Peeking does not affect `hot`.
                       # Peeking does not affect the length.
                   ], 
                   restore_hot,                
               ],
           ),
       ),
       cb.if_with(
           # Did the user call push?
           cmd_eq_2,
           cb.if_with( len_eq_max_queue_len, raise_err,  # The queue is full: overflow.
               [  # The queue is not full. Proceed.
                   lower_err,
                   # We need to check which flow this value should be pushed to.
                   flow_handles,
                   cb.if_with( err_eq_0,
                       # If no stats component is provided,
                       # just increment the active length.
                       (len_incr
                           if not stats
                           else cb.par(
                               # If a stats component is provided,
                               # Increment the active length and also
                               # tell the stats component what flow we pushed.
                               len_incr,
                               (
                                   cb.static_invoke(stats, in_flow=flow.out)
                                   if static
                                   else cb.invoke(stats, in_flow=flow.out)
                               ),
                           )
                       ),
                   ),
               ],
           ),
       ),
   ) 


   return pifo


def build():
   """Top-level function to build the program."""
   prog = cb.Builder()
   n_flows = 2
   sub_fifos = []
   for n in range(n_flows):
       name = "fifo" + str(n)
       sub_fifo = fifo.insert_fifo(prog, name, QUEUE_LEN_FACTOR)
       sub_fifos.append(sub_fifo)

   pifo = insert_pifo(prog, "pifo", sub_fifos, [0, 200, 400], n_flows)
   qc.insert_main(prog, pifo, 20)
   return prog.program

if __name__ == "__main__":
   build().emit()