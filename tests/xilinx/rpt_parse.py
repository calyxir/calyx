#!/usr/bin/env python3

# Test RPT parsing in fud
import argparse
import os
from pathlib import Path
from fud.stages.vivado.extract import place_and_route_extract

# import json

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Test RPT parsing in fud")
    parser.add_argument("--dir", type=str, help="RPT directory", required=True)
    args = parser.parse_args()

    root = Path(os.path.dirname(__file__)) / "rpt"

    data = place_and_route_extract(
        root,
        args.dir,
        "impl.rpt",
        "timing.rpt",
        "synth.rpt",
    )

    # Output data as string
    print(data)
