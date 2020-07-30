#!/usr/bin/env python3

import sys
from pathlib import Path
import json
import pandas as pd

def main():
  data = []
  for directory in sys.argv[1:]:
    for f in Path(directory).glob('*'):
        if f.is_dir():
          hls = json.load((f / "hls.json").open())
          futil = json.load((f / "futil.json").open())
          data.append({
            'benchmark': f.stem,
            'type': 'hls',
            'lut': hls['LUT'],
            'dsp': hls['DSP'],
            'source': directory
          })
          data.append({
            'benchmark': f.stem,
            'type': 'hls_total',
            'lut': hls['TOTAL_LUT'],
            'dsp': hls['DSP'],
            'source': directory
          })
          data.append({
            'benchmark': f.stem,
            'type': 'futil',
            'lut': futil['LUT'],
            'dsp': futil['DSP'],
            'source': directory
          })
  df = pd.DataFrame(data)
  print(df.to_csv(index=False))

if __name__ == "__main__":
    main()
