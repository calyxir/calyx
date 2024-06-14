import fifo
import calyx.builder as cb
import calyx.queue_call as qc

QUEUE_LEN_FACTOR = 4

def invoke_subqueue(pifo, sub_fifo, num, cmd, value, ans, err) -> cb.invoke:
    """Invokes the cell {queue_cell} with:
    {cmd} passed by value
    {value} passed by value
    {ans} passed by reference
    {err} passed by reference
    """
    name = "queue_" + str(num)
    queue_cell = pifo.cell(name, sub_fifo)
    return cb.invoke(
        queue_cell,
        in_cmd=cmd,
        in_value=value,
        ref_ans=ans,
        ref_err=err,
    )

def insert_test(prog,
    name,
    fifos,
    boundary,
    n_flows, # the number of flows
    queue_len_factor=QUEUE_LEN_FACTOR,
    ):
  pifo: cb.ComponentBuilder = prog.component(name)

  hot = pifo.reg(32)
  cmd = pifo.input("cmd", 2) # the size in bits is 2

  value = pifo.input("value", 32)  # The value to push to the queue
    
  ans = pifo.reg(32, "ans", is_ref=True)
  # If the user wants to pop, we will write the popped value to `ans`.

  err = pifo.reg(1, "err", is_ref=True)
  # We'll raise this as a general error flag for overflow and underflow.
  reset_hot = pifo.reg_store(hot, 0, "reset_hot") # hot := 0
  hot_eq_n = pifo.eq_use(hot.out, n_flows-1)
  flip_hot = pifo.incr(hot)

  handles = []
  for n in range(n_flows):
    handle = cb.if_with(pifo.eq_use(hot.out, n), # const(n, 32)
    invoke_subqueue(pifo, fifos[n], n, cmd, value, ans, err))
    handles.append(handle)

  pifo.control += cb.par(
    handles,
    cb.if_with(hot_eq_n, flip_hot, reset_hot),

  )

def build():
  """Top-level function to build the program."""
  prog = cb.Builder()
  n_flows = 3
  sub_fifos = []
  for n in range(n_flows):
      name = "fifo" + str(n)
      sub_fifo = fifo.insert_fifo(prog, name, QUEUE_LEN_FACTOR)
      sub_fifos.append(sub_fifo)

  insert_test(prog, "pifo", sub_fifos, 200, n_flows)
  return prog.program


if __name__ == "__main__":
  build().emit()