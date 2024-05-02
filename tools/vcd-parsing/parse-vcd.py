import json
import sys
from pyDigitalWaveTools.vcd.parser import VcdParser

if len(sys.argv) > 1:
    fname = sys.argv[1]
else:
    print('Give me a vcd file to parse')
    sys.exit(-1)

with open(fname) as vcd_file:
    vcd = VcdParser()
    vcd.parse(vcd_file)
    data = vcd.scope.toJson()
    print(json.dumps(data, indent=4, sort_keys=True))
