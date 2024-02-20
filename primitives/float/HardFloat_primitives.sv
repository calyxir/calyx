`ifndef __HARDFLOAT_PRIMITIVES_VI__
`define __HARDFLOAT_PRIMITIVES_VI__

/*============================================================================

This Verilog source file is part of the Berkeley HardFloat IEEE Floating-Point
Arithmetic Package, Release 1, by John R. Hauser.

Copyright 2019 The Regents of the University of California.  All rights
reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

 1. Redistributions of source code must retain the above copyright notice,
    this list of conditions, and the following disclaimer.

 2. Redistributions in binary form must reproduce the above copyright notice,
    this list of conditions, and the following disclaimer in the documentation
    and/or other materials provided with the distribution.

 3. Neither the name of the University nor the names of its contributors may
    be used to endorse or promote products derived from this software without
    specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS "AS IS", AND ANY
EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE, ARE
DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

=============================================================================*/

/*----------------------------------------------------------------------------
*----------------------------------------------------------------------------*/

module
    reverse#(parameter width = 1) (
        input [(width - 1):0] in, output [(width - 1):0] out
    );

    genvar ix;
    generate
        for (ix = 0; ix < width; ix = ix + 1) begin :Bit
            assign out[ix] = in[width - 1 - ix];
        end
    endgenerate

endmodule

/*----------------------------------------------------------------------------
*----------------------------------------------------------------------------*/

module
    lowMaskHiLo#(
        parameter inWidth = 1,
        parameter topBound = 1,
        parameter bottomBound = 0
    ) (
        input [(inWidth - 1):0] in,
        output [(topBound - bottomBound - 1):0] out
    );

    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    localparam numInVals = 1<<inWidth;
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    wire signed [numInVals:0] c;
    assign c[numInVals] = 1;
    assign c[(numInVals - 1):0] = 0;
    wire [(topBound - bottomBound - 1):0] reverseOut =
        (c>>>in)>>(numInVals - topBound);
    reverse#(topBound - bottomBound) reverse(reverseOut, out);

endmodule

/*----------------------------------------------------------------------------
*----------------------------------------------------------------------------*/

module
    lowMaskLoHi#(
        parameter inWidth = 1,
        parameter topBound = 0,
        parameter bottomBound = 1
    ) (
        input [(inWidth - 1):0] in,
        output [(bottomBound - topBound - 1):0] out
    );

    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    localparam numInVals = 1<<inWidth;
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    wire signed [numInVals:0] c;
    assign c[numInVals] = 1;
    assign c[(numInVals - 1):0] = 0;
    wire [(bottomBound - topBound - 1):0] reverseOut =
        (c>>>~in)>>(topBound + 1);
    reverse#(bottomBound - topBound) reverse(reverseOut, out);

endmodule

/*----------------------------------------------------------------------------
*----------------------------------------------------------------------------*/

module
    countLeadingZeros#(parameter inWidth = 1, parameter countWidth = 1) (
        input [(inWidth - 1):0] in, output [(countWidth - 1):0] count
    );

    wire [(inWidth - 1):0] reverseIn;
    reverse#(inWidth) reverse_in(in, reverseIn);
    wire [inWidth:0] oneLeastReverseIn =
        {1'b1, reverseIn} & ({1'b0, ~reverseIn} + 1);
    genvar ix;
    generate
        for (ix = 0; ix <= inWidth; ix = ix + 1) begin :Bit
            wire [(countWidth - 1):0] countSoFar;
            if (ix == 0) begin
                assign countSoFar = 0;
            end else begin
                assign countSoFar =
                    Bit[ix - 1].countSoFar | (oneLeastReverseIn[ix] ? ix : 0);
                if (ix == inWidth) assign count = countSoFar;
            end
        end
    endgenerate

endmodule

/*----------------------------------------------------------------------------
*----------------------------------------------------------------------------*/

module
    compressBy2#(parameter inWidth = 1) (
        input [(inWidth - 1):0] in, output [((inWidth - 1)/2):0] out
    );

    localparam maxBitNumReduced = (inWidth - 1)/2;
    genvar ix;
    generate
        for (ix = 0; ix < maxBitNumReduced; ix = ix + 1) begin :Bit
            assign out[ix] = |in[(ix*2 + 1):ix*2];
        end
    endgenerate
    assign out[maxBitNumReduced] = |in[(inWidth - 1):maxBitNumReduced*2];

endmodule

/*----------------------------------------------------------------------------
*----------------------------------------------------------------------------*/

module
    compressBy4#(parameter inWidth = 1) (
        input [(inWidth - 1):0] in, output [(inWidth - 1)/4:0] out
    );

    localparam maxBitNumReduced = (inWidth - 1)/4;
    genvar ix;
    generate
        for (ix = 0; ix < maxBitNumReduced; ix = ix + 1) begin :Bit
            assign out[ix] = |in[(ix*4 + 3):ix*4];
        end
    endgenerate
    assign out[maxBitNumReduced] = |in[(inWidth - 1):maxBitNumReduced*4];

endmodule

`endif /* __HARDFLOAT_PRIMITIVES_VI__ */