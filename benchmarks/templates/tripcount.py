#!/usr/bin/env python3
import sys
import json
from pathlib import Path

class TripCount():
    def __init__(self, name):
        self.name = name
        self.current = 0
        self.account = []

    def incr(self):
        self.current += 1

    def reset(self):
        self.account.append(self.current)
        self.current = 0

    def output(self):
        mx = max(self.account)
        mn = min(self.account)
        avg = round(sum(self.account) / len(self.account))
        print(f'{self.name}:')
        print(f'\tdecor "#pragma HLS loop_tripcount min={mn} max={mx} avg={avg}"')
        # print(f'\t{self.account}')


if __name__ == "__main__":
    benchmark = sys.argv[1]
    size = sys.argv[2]

    # read json file
    template = json.load(Path(f"{size}_polybench/linear-algebra-{benchmark}.template").open())
    t = template['key']

    if benchmark == "gramschmidt":
        while1 = TripCount('while1')
        for k in range(t['N']):
            for i in range(t['M']):
                pass
            for i in range(t['M']):
                pass
            j = k + 1
            while (j < t['N']):
                while1.incr()
                for i in range(t['M']):
                    pass
                for i in range(t['M']):
                    pass
                j += 1
            while1.reset()
        while1.output()

    elif benchmark == "cholesky":
        while1 = TripCount('while1')
        while2 = TripCount('while2')
        while3 = TripCount('while3')
        for i in range(t['N']):
            j = 0
            while (j < i):
                while1.incr()
                k = 0
                while (k < j):
                    while2.incr()
                    k += 1
                while2.reset()
                j += 1
            while1.reset()
            k = 0
            while (k < i):
                while3.incr()
                k += 1
            while3.reset()
        while1.output()
        while2.output()
        while3.output()

    elif benchmark == "durbin":
        while1 = TripCount('while')
        for k in range(1, t['N']):
            i = 0
            while (i < k):
                while1.incr()
                i += 1
            while1.reset()
        while1.output()

    elif benchmark == "trisolv":
        while1 = TripCount('while1')
        for i in range(t['N']):
            j = 0
            while (j < i):
                while1.incr()
                j += 1
            while1.reset()
        while1.output()

    elif benchmark == "syrk":
        while1 = TripCount('while1')
        while2 = TripCount('while2')
        for i in range(t['N']):
            j = 0
            while (j <= i):
                while1.incr()
                j += 1
            while1.reset()
            j2 = 0
            while (j2 <= i):
                while2.incr()
                for k in range(t['M']):
                    pass
                j2 += 1
            while2.reset()
        while1.output()
        while2.output()

    elif benchmark == "syr2k":
        while1 = TripCount('while1')
        while2 = TripCount('while2')
        for i in range(t['N']):
            j = 0
            while (j <= i):
                while1.incr()
                j += 1
            while1.reset()
            j2 = 0
            while (j2 <= i):
                while2.incr()
                for k in range(t['M']):
                    pass
                j2 += 1
            while2.reset()
        while1.output()
        while2.output()

    elif benchmark == "trmm":
        while1 = TripCount('while')
        for i in range(t['M']):
            for j in range(t['N']):
                k = i + 1
                while (k < t['M']):
                    while1.incr()
                    k += 1
                while1.reset()
        while1.output()

    elif benchmark == "symm":
        while1 = TripCount('while')
        for i in range(t['M']):
            for j in range(t['N']):
                for k in range(i):
                    while1.incr()
                while1.reset()
        while1.output()

    elif benchmark == "lu":
        for1 = TripCount('for1')
        while1 = TripCount('while1')
        while1_1 = TripCount('while1_1')
        while2 = TripCount('while2')
        while2_1 = TripCount('while2_1')
        for i in range(t['N']):
            for1.incr()
            j = 0
            while (j < i):
                while1.incr()
                k = 0
                while (k < j):
                    while1_1.incr()
                    k += 1
                while1_1.reset()
                j += 1
            while1.reset()

            j = i
            while (j < t['N']):
                while2.incr()
                k = 0
                while (k < i):
                    while2_1.incr()
                    k += 1
                while2_1.reset()
                j += 1
            while2.reset()
        for1.reset()
        for1.output()
        while1.output()
        while1_1.output()
        while2.output()
        while2_1.output()
    else:
        print(f"{benchmark} has no tripcount registered.")
