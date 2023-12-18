# Import the sdn module, which is one level up.
import sys
import os

sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import sdn

if __name__ == "__main__":
    sdn.build(static=True).emit()
