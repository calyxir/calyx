import numpy as np

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

