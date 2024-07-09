import os
import sys
import inspect

currentdir = os.path.dirname(os.path.abspath(inspect.getfile(inspect.currentframe())))
parentdir = os.path.dirname(currentdir)
sys.path.insert(0, parentdir)

from gen_strict_or_rr import build

if __name__ == "__main__":
    """Invoke the top-level function to build the program, with 5 flows."""
    build(5, False).emit()