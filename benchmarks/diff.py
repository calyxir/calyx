#!/usr/bin/env python3

import sys

def approxeq(a, b):
    if str(a) == "nan" and str(b) == "nan":
        return True

    return float("{:.5g}".format(a)) == float("{:.5g}".format(b))

def main():
    filenameA = sys.argv[1]
    filenameB = sys.argv[2]

    with open(filenameA, "r") as fileA:
        with open(filenameB, "r") as fileB:
            cleanA = list(map(lambda x: x.strip(), fileA.readlines()))
            cleanB = list(map(lambda x: x.strip(), fileB.readlines()))
            pad = 5
            valWidth = max(map(lambda x: len(x), list(cleanA) + list(cleanB))) + pad
            if len(cleanA) != len(cleanB):
                print("Files different length!")
                exit(-1)
            idxWidth = len(str(len(cleanA)))
            failed = False
            for (i, (a, b)) in enumerate(zip(cleanA, cleanB)):
                if not(approxeq(float(a), float(b))):
                    failed = True
                    print("{}: {}|{}".format(str(i).ljust(idxWidth),
                                             a.ljust(valWidth),
                                             b.rjust(valWidth)))
    if failed:
        if input("Human, tell me. Are these the same?: ") != "y":
            exit(-1)

if __name__ == "__main__":
    main()
