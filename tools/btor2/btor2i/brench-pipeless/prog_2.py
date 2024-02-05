import sys

if __name__ == '__main__':
    line_count = 0
    # on the first testcase this gets the values prog_2.py, 12, 34, tests/input_1.txt in that order. the text file contains 3 lines so the final value is 6.
    # on the second testcase this gets the values prog_2.py, tests/input_2.txt in that order. the text file contains 2 lines so the final value is 3.
    for a in sys.argv:
        if '.txt' in a:
            with open(a, 'r') as input_path:
                for line in input_path.readlines():
                    if line.strip():
                        line_count += 1
        else:
            line_count += 1

    for line in sys.stdin:
        pass

print(f"total_lines: {line_count}", file=sys.stderr)
