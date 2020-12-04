import json
import numpy as np
from pathlib import Path

# Converts `val` into a bitstring that is `bw` characters wide
def bitstring(val, bw):
    # first truncate val by `bw` bits
    val &= (2**bw - 1)
    return '{:x}'.format(val)

def bitstring2(val, bw):
    #first truncate val by `bw` bits
    val = val[len(val)-bw:len(val)]
    numval = int(val,base=2)
    return '{:x}'.format(numval)


def parse_dat(path):
    with path.open('r') as f:
        lines = f.readlines()
        arr = np.array(list(map(lambda v: int(v.strip(), 16), lines)))
        return arr

def decimal_to_fixed_p (num, width, int_bit, frac_bit):
    '''
    given number, width, integer bit and fractinal bit,
    returns the fixed point representation.
    example: decimal_to_fixed_p (11.125,8,5,3) returns 01011001 = 2^3+2^1+2^0+2^(-3)
    precondition: There is no overflow 
    (integer part of the number should be representable with int_bit number of bits )
    '''
    # seperate to int and fractional parts
    intg , frac = str(num).split(".") 
    int_b = np.binary_repr(int(intg), width = int_bit)
    frac = "0."+frac
    # multiply fractional part with 2**frac_bit to turn into integer
    # frac= int(float(frac) * float(2**frac_bit))
    frac = float(frac) * float(2**frac_bit)
    _, f = str(frac).split(".") 
    # raises Exception when the number can't be represented as fixed numbers
    if f != "0":
        raise Exception("number can't be represented as fixedpoint numbers")
    frac = int(frac)
    frac_b = np.binary_repr(frac, width = frac_bit)
    r = int_b + frac_b 
    return r


def fixed_p_to_decimal (fp, width, int_bit, frac_bit):
    '''
    given fixedpoint representation, width, integer bit and fractinal bit,
    returns the number.
    example: fixed_p_to_decimal ('01011001',8,5,3) returns 11.125
    '''
    int_b = fp[:int_bit]
    frac_b = fp[int_bit:width]
    int_num = int(int_b, 2) 
    frac = int(frac_b, 2) 
    frac_num = float(frac / 2**(frac_bit))
    num = float(int_num +frac_num)
    return num


# go through the json data and create a file for each key,
# flattening the data and then converting it to bitstrings

# original version 
# def convert2dat(output_dir, data, extension):
#     output_dir = Path(output_dir)
#     shape = {}
#     for k, item in data.items():
#         path = output_dir / f"{k}.{extension}"
#         path.touch()
#         arr = np.array(item["data"])
#         shape[k] = {"shape": list(arr.shape), "bitwidth": item["bitwidth"]}
#         with path.open('w') as f:
#             for v in arr.flatten():
#                 f.write(bitstring(v, item["bitwidth"]) + "\n")

#     # commit shape.json file
#     shape_json_file = output_dir / "shape.json"
#     with shape_json_file.open('w') as f:
#         json.dump(shape, f, indent=2)


# revised version 
def convert2dat(output_dir, data, extension):
    output_dir = Path(output_dir)
    shape = {}
    for k, item in data.items():
        path = output_dir / f"{k}.{extension}"
        path.touch()
        arr = np.array(item["data"])
        # fixedpoint informations arre given as array [int bit, fractional bit]
        bits = np.array(item["fixedpoint"])
        shape[k] = {"shape": list(arr.shape), "bitwidth": item["bitwidth"], "fixedpoint": list(bits.shape)}
        # if empty, then bitstrings are directly computed
        if list(bits.shape)[0] ==0:
            with path.open('w') as f:
                for v in arr.flatten():
                    f.write(bitstring(v, item["bitwidth"]) + "\n")
        # if given int bit, fract bit then fixedpoint bitstrings are computed
        elif list(bits.shape)[0] ==2:
            with path.open('w') as f:
                for v in arr.flatten():
                    bs =  decimal_to_fixed_p(v, item["bitwidth"], bits[0], bits[1])
                    f.write( bitstring2(bs, item["bitwidth"])+ "\n")
        # other cases are not allowed
        else:
            raise Exception("give [] if not fixedpoint, [intbit, fractbit] if fixedpoint")

    # commit shape.json file
    shape_json_file = output_dir / "shape.json"
    with shape_json_file.open('w') as f:
        json.dump(shape, f, indent=2)


# converts a directory of *.dat files back into a json file
# TODO: Figure out extention for this 
def convert2json(input_dir, extension):
    input_dir = Path(input_dir)
    data = {}
    shape_json_path = input_dir / "shape.json"
    shape_json = None
    if shape_json_path.exists():
        shape_json = json.load(shape_json_path.open('r'))

    # TODO: change to use shape json
    for f in input_dir.glob(f'*.{extension}'):
        arr = parse_dat(f)
        if shape_json is not None and shape_json.get(f.stem) is not None and shape_json[f.stem]["shape"] != [0]:
            try:
                arr = arr.reshape(tuple(shape_json[f.stem]["shape"]))
            except Exception:
                raise Exception(f.stem)
            name = f.stem
            # TODO: this is probably important, figure out why (I think it was used for Dahlia benchmarks)
            # if '_int' in name:
            data[name] = arr.tolist()
    return data
