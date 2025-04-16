# pylint: disable=import-error
import sys
import fifo
import calyx.builder as cb
import calyx.queue_call as qc
import strict_and_rr_queues.gen_strict_or_rr as strict_or_rr
import json


# def unroll(data):
#   """Need to recurse until we hit leaves, and then build up"""
#   for key, value in data.items():
#     if key == "class":
#       name = "fifo_" + value
#       fifos.append(fifo.insert_fifo(prog, name))
#     else:
#       # need to continue recursing to find leaf
#       unroll(value)

def create(data, lower, upper):
  """
  Recursively creates the pifo
  """
  if isinstance(data, dict):
    for key, val in data.items():
      if key == "FIFO":
        classes = ""
        for clss in val:
          classes += clss
        name = "fifo_" + classes
        return fifo.insert_fifo(prog, name)
      elif key == "RR":
        num_children = len(val)
        interval = (upper - lower) // num_children
        boundaries = [lower]
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
          children.append(create(child, l, u))
          l = u

        pifo_rr = strict_or_rr.insert_queue(
        prog,
        "pifo_rr",
        children,
        boundaries, # boundaries comes from the range it is given divided equally amongst num of flows
        num_children,
        [],
        True,
        )
      elif key == "Strict":
                num_children = len(val)
        interval = (upper - lower) // num_children
        boundaries = [lower]
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
          children.append(create(child, l, u))
          l = u

        pifo_strict = strict_or_rr.insert_queue(
        prog,
        "pifo_strict",
        children,
        boundaries,
        num_children,
        [n for n in range(num_children)],
        False,
        )
  elif isinstance(data, list):
    return create(child) for child in data



def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()

    # for now assume sys.argv[2] is the name of the json file

    data = json.loads(sys.argv[2])
    root = create(data, 0, 400)

    qc.insert_main(prog, root, num_cmds, keepgoing=keepgoing)
    return prog,program


if __name__ == "__main__":
    build().emit()
