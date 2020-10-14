import sys
import json
from .parse import parse
from .gen_futil import emit
from .interp import interp, InterpError


def main():
    with open(sys.argv[1]) as f:
        txt = f.read()
    ast = parse(txt)
    out = emit(ast)


if __name__ == '__main__':
    main()
