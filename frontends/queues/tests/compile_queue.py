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

def create(data, lower, upper, prog):
  """
  Recursively creates the pifo
  """
  if isinstance(data, dict):
    for key, val in data.items():
      if key == "FIFO":
        return fifo.insert_fifo(prog, "fifo")
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
          children.append(create(child, l, u, prog))
          l = u

        return strict_or_rr.insert_queue(
        prog,
        "pifo_rr",
        True,
        children,
        fi.insert_boundary_flow_inference(prog, "fi_rr", boundaries), # could be problem with using the same name for different flow inferences?
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
          children.append(create(child, l, u, prog))
          l = u

        return strict_or_rr.insert_queue(
        prog,
        "pifo_strict",
        False,
        children,
        fi.insert_boundary_flow_inference(prog, "fi_strict", boundaries),
        order = [n for n in range(num_children)],
        )
  elif isinstance(data, list):
    for child in data:
      create(child)



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

    root = create(data, 0, 400, prog)

    qc.insert_main(prog, root, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
