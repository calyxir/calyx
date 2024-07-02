# pylint: disable=import-error
import os
import sys
import inspect

currentdir = os.path.dirname(os.path.abspath(inspect.getfile(inspect.currentframe())))
parentdir = os.path.dirname(currentdir)
sys.path.insert(0, parentdir) 

import fifo
import calyx.builder as cb
import calyx.queue_call as qc
import roundrobin

# This determines the maximum possible length of the queue:
# The max length of the queue will be 2^QUEUE_LEN_FACTOR.
QUEUE_LEN_FACTOR = 4

def build():
   """Top-level function to build the program."""
   prog = cb.Builder()
   n_flows = 4
   sub_fifos = []
   for n in range(n_flows):
       name = "fifo" + str(n)
       sub_fifo = fifo.insert_fifo(prog, name, QUEUE_LEN_FACTOR)
       sub_fifos.append(sub_fifo)

   pifo = roundrobin.insert_rr_pifo(prog, "pifo", sub_fifos, [0, 100, 200, 300, 400], n_flows)
   qc.insert_main(prog, pifo, 20000)
   return prog.program

if __name__ == "__main__":
   build().emit()