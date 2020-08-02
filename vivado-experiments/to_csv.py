#!/usr/bin/env python3

import sys
from pathlib import Path
import json
import pandas as pd

def main():
  result = []
  for directory in sys.argv[1:]:
    for f in Path(directory).glob('*'):
        if f.is_dir():
          data = json.load((f / "data.json").open())
          hls = data['hls']
          futil = data['futil']
          result.append({
            'benchmark': f.stem,
            'type': 'hls',
            'lut': hls['LUT'],
            'dsp': hls['DSP'],
            'latency': hls['AVG_LATENCY'],
            'source': directory
          })
          result.append({
            'benchmark': f.stem,
            'type': 'hls_total',
            'lut': hls['TOTAL_LUT'],
            'dsp': hls['DSP'],
            'latency': hls['AVG_LATENCY'],
            'source': directory
          })
          result.append({
            'benchmark': f.stem,
            'type': 'futil',
            'lut': futil['LUT'],
            'dsp': futil['DSP'],
            'latency': 0,
            'source': directory
          })
  df = pd.DataFrame(result)
  print(df.to_csv(index=False))

if __name__ == "__main__":
    main()
