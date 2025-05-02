from compile_queue import build
"""
A test for the compiler-generated version of the same round robin tree with 2 
flows as in rr_2flows.py (and the rio program rr_2_classes.sched in the packet
scheduling repo)
"""

if __name__ == "__main__":
  build("rr_2flows.json").emit()