import sys

def debug(*args, **kwargs):
    print(*args, **kwargs, file=sys.stderr)
