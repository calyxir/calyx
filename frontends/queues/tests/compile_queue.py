# pylint: disable=import-error
import sys
import calyx.builder as cb
import queues.queue_call as qc
import queues.strict_or_rr as strict_or_rr
import queues.fifo as fifo
import queues.flow_inference as fi
import json
import os

# def unroll(data):
#   """Need to recurse until we hit leaves, and then build up"""
#   for key, value in data.items():
#     if key == "class":
#       name = "fifo_" + value
#       fifos.append(fifo.insert_fifo(prog, name))
#     else:
#       # need to continue recursing to find leaf
#       unroll(value)
rr_id = 0
strict_id = 0

def create(data, lower, upper, prog, fifo_queue):
  """
  Recursively creates the pifo
  """
  global rr_id
  global strict_id
  if isinstance(data, dict):
    for key, val in data.items():
      if key == "FIFO":
        return fifo_queue
      elif key == "RR":
        num_children = len(val)
        interval = (upper - lower) // num_children
        boundaries = []
        for i in range(1, num_children):
          boundaries.append(lower + (interval * i))
        boundaries.append(upper)

        children = []
        l = lower
        u = upper # keep in mind case where there are only 2 children, could lead to a bug with rounding error
        for child in range(num_children):
          u = l + interval
          if child == num_children - 1:
            u = upper
          children.append(create(val[child], l, u, prog, fifo_queue))
          l = u

        rr_id += 1
        return strict_or_rr.insert_queue(
        prog,
        f"pifo_rr{rr_id}",
        True,
        children,
        fi.insert_boundary_flow_inference(prog, f"fi_rr{rr_id}", boundaries),
        )
      elif key == "Strict":
        num_children = len(val)
        interval = (upper - lower) // num_children
        boundaries = []
        for i in range(1, num_children):
          boundaries.append(lower + (interval * i))
        boundaries.append(upper)

        children = []
        l = lower
        u = upper # keep in mind case where there are only 2 children, could lead to a bug with rounding error
        for child in range(num_children):
          u = l + interval
          if child == num_children - 1:
            u = upper
          children.append(create(val[child], l, u, prog, fifo_queue))
          l = u

        strict_id += 1
        return strict_or_rr.insert_queue(
        prog,
        f"pifo_strict{strict_id}",
        False,
        children,
        fi.insert_boundary_flow_inference(prog, f"fi_strict{strict_id}", boundaries),
        order = [n for n in range(num_children)],
        )
  # elif isinstance(data, list):
  #   for child in data:
  #     create(child)



def build(json_file):
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()

    #data = json.loads(json_file)
    base_dir = os.path.dirname(__file__)
    file_path = os.path.join(base_dir, json_file)
    with open(file_path) as f:
      data = json.load(f)

    fifo_queue = fifo.insert_fifo(prog, "fifo")
    root = create(data, 0, 400, prog, fifo_queue)

    qc.insert_main(prog, root, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
