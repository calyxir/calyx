import sys
import json
from .parse import parse
from .interp import interp, InterpError


def main():
    with open(sys.argv[1]) as f:
        txt = f.read()
    with open(sys.argv[2]) as f:
        data = json.load(f)
    ast = parse(txt)

    try:
        out = interp(ast, data)
    except InterpError as exc:
        print(str(exc), file=sys.stderr)
        sys.exit(1)

    print(json.dumps(out, sort_keys=True, indent=2))


if __name__ == '__main__':
    main()
