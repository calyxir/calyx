from queues.compile_queue import build

"""
A test for the compiler-generated version of the same fifo as in fifo_test.py
(and the rio program fifo_1_class_sugar.sched in the packet scheduling repo)
"""

if __name__ == "__main__":
    build("fifo.json").emit()
