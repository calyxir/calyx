from queues.compile_queue import build

"""A test for the compiler-generated version of the same strict tree with 3 flows
as in strict_3flows.py (and the rio program strict_n_classes.sched in the packet
scheduling repo)"""

if __name__ == "__main__":
    build("strict_3flows.json").emit()
