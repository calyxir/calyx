`ifndef __HARDFLOAT_DIVSQRTRECFN_V__
`define __HARDFLOAT_DIVSQRTRECFN_V__

`include "primitives/float-point/HardFloat_primitives.sv"

module
  fNToRecFN #(
    parameter expWidth = 3, 
    parameter sigWidth = 3,
    parameter inputWidth = 6,
    parameter outputWidth = 7
) (
    input [inputWidth-1:0] in_,
    output [outputWidth-1:0] out
);
`include "primitives/float-point/HardFloat_localFuncs.vi"

  /*------------------------------------------------------------------------
  *------------------------------------------------------------------------*/
  localparam normDistWidth = clog2(sigWidth);
  /*------------------------------------------------------------------------
  *------------------------------------------------------------------------*/
  wire sign;
  wire [(expWidth - 1):0] expIn;
  wire [(sigWidth - 2):0] fractIn;
  assign {sign, expIn, fractIn} = in_;
  wire isZeroExpIn = (expIn == 0);
  wire isZeroFractIn = (fractIn == 0);
  /*------------------------------------------------------------------------
  *------------------------------------------------------------------------*/
  wire [(normDistWidth - 1):0] normDist;
  countLeadingZeros#(sigWidth - 1, normDistWidth)
      countLeadingZeros(fractIn, normDist);
  wire [(sigWidth - 2):0] subnormFract = (fractIn<<normDist)<<1;
  wire [expWidth:0] adjustedExp =
      (isZeroExpIn ? normDist ^ ((1<<(expWidth + 1)) - 1) : expIn)
          + ((1<<(expWidth - 1)) | (isZeroExpIn ? 2 : 1));
  wire isZero = isZeroExpIn && isZeroFractIn;
  wire isSpecial = (adjustedExp[expWidth:(expWidth - 1)] == 'b11);
  /*------------------------------------------------------------------------
  *------------------------------------------------------------------------*/
  wire [expWidth:0] exp;
  assign exp[expWidth:(expWidth - 2)] =
      isSpecial ? {2'b11, !isZeroFractIn}
          : isZero ? 3'b000 : adjustedExp[expWidth:(expWidth - 2)];
  assign exp[(expWidth - 3):0] = adjustedExp;
  assign out = {sign, exp, isZeroExpIn ? subnormFract : fractIn};

endmodule

`endif /* __HARDFLOAT_DIVSQRTRECFN_V__ */
