from compile_queue import build
"""
A test for the compiler-generated version of the same pifo tree as in 
pifo_tree_test.py (and the rio program rr_hier.sched in the packet
scheduling repo)
"""

if __name__ == "__main__":
  build("pifo_tree.json").emit()