from queues.compile_queue import build

"""
A test for the compiler-generated version of the round robin with only one child
that is just a fifo of the union of 2 classes. This is the Rio program rr_2_classes.sched 
in the packet scheduling repo, which is rr[fifo[union[A, B]]]).
"""

if __name__ == "__main__":
    build("rr_union.json").emit()
