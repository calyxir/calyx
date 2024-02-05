import sys

if __name__ == '__main__':
    for a in sys.argv:
        pass

    stdin_count = 0  # stdin is empty so this remains 0 always
    for line in sys.stdin:
        if line:
            stdin_count += 1

print(f"total_lines: {stdin_count}", file=sys.stderr)
