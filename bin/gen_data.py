#!/usr/bin/env python3

import json
import numpy as np
import sys
from pathlib import Path

def main(path):
    template = json.load(path.open())
    for key in template:
        template[key]['data'] = np.random.randint(100, size=template[key]['data']).tolist()
    print(json.dumps(template, indent=2))

if __name__ == "__main__":
    filename = Path(sys.argv[1])
    if filename.exists():
        main(filename)
    else:
        print(f"{filename} doesn't exist.")
        exit(1)
