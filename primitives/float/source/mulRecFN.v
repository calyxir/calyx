`ifndef __HARDFLOAT_MULRECFN_V__
`define __HARDFLOAT_MULRECFN_V__

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

`include "primitives/float/source/HardFloat_consts.vi"
`include "primitives/float/source/HardFloat_specialize.vi"
`include "primitives/float/source/isSigNaNRecFN.v"

/*----------------------------------------------------------------------------
*----------------------------------------------------------------------------*/

module
    mulRecFNToFullRaw#(parameter expWidth = 3, parameter sigWidth = 3) (
        input [(`floatControlWidth - 1):0] control,
        input [(expWidth + sigWidth):0] a,
        input [(expWidth + sigWidth):0] b,
        output invalidExc,
        output out_isNaN,
        output out_isInf,
        output out_isZero,
        output out_sign,
        output signed [(expWidth + 1):0] out_sExp,
        output [(sigWidth*2 - 1):0] out_sig
    );

    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    wire isNaNA, isInfA, isZeroA, signA;
    wire signed [(expWidth + 1):0] sExpA;
    wire [sigWidth:0] sigA;
    recFNToRawFN#(expWidth, sigWidth)
        recFNToRawFN_a(a, isNaNA, isInfA, isZeroA, signA, sExpA, sigA);
    wire isSigNaNA;
    isSigNaNRecFN#(expWidth, sigWidth) isSigNaN_a(a, isSigNaNA);
    wire isNaNB, isInfB, isZeroB, signB;
    wire signed [(expWidth + 1):0] sExpB;
    wire [sigWidth:0] sigB;
    recFNToRawFN#(expWidth, sigWidth)
        recFNToRawFN_b(b, isNaNB, isInfB, isZeroB, signB, sExpB, sigB);
    wire isSigNaNB;
    isSigNaNRecFN#(expWidth, sigWidth) isSigNaN_b(b, isSigNaNB);
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    wire notSigNaN_invalidExc = (isInfA && isZeroB) || (isZeroA && isInfB);
    wire notNaN_isInfOut = isInfA || isInfB;
    wire notNaN_isZeroOut = isZeroA || isZeroB;
    wire notNaN_signOut = signA ^ signB;
    wire signed [(expWidth + 1):0] common_sExpOut =
        sExpA + sExpB - (1<<expWidth);
    wire [(sigWidth*2 - 1):0] common_sigOut = sigA * sigB;
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    assign invalidExc = isSigNaNA || isSigNaNB || notSigNaN_invalidExc;
    assign out_isInf = notNaN_isInfOut;
    assign out_isZero = notNaN_isZeroOut;
    assign out_sExp = common_sExpOut;
`ifdef HardFloat_propagateNaNPayloads
    assign out_isNaN = isNaNA || isNaNB || notSigNaN_invalidExc;
    wire signNaN;
    wire [(sigWidth - 2):0] fractNaN;
    propagateFloatNaN_mul#(sigWidth)
        propagateNaN(
            control,
            isNaNA,
            signA,
            sigA[(sigWidth - 2):0],
            isNaNB,
            signB,
            sigB[(sigWidth - 2):0],
            signNaN,
            fractNaN
        );
    assign out_sign = out_isNaN ? signNaN : notNaN_signOut;
    assign out_sig =
        out_isNaN ? {1'b1, fractNaN}<<(sigWidth - 1) : common_sigOut;
`else
    assign out_isNaN = isNaNA || isNaNB;
    assign out_sign = notNaN_signOut;
    assign out_sig = common_sigOut;
`endif

endmodule

/*----------------------------------------------------------------------------
*----------------------------------------------------------------------------*/

module
    mulRecFNToRaw#(parameter expWidth = 3, parameter sigWidth = 3) (
        input [(`floatControlWidth - 1):0] control,
        input [(expWidth + sigWidth):0] a,
        input [(expWidth + sigWidth):0] b,
        output invalidExc,
        output out_isNaN,
        output out_isInf,
        output out_isZero,
        output out_sign,
        output signed [(expWidth + 1):0] out_sExp,
        output [(sigWidth + 2):0] out_sig
    );

    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    wire isNaNA, isInfA, isZeroA, signA;
    wire signed [(expWidth + 1):0] sExpA;
    wire [sigWidth:0] sigA;
    recFNToRawFN#(expWidth, sigWidth)
        recFNToRawFN_a(a, isNaNA, isInfA, isZeroA, signA, sExpA, sigA);
    wire isSigNaNA;
    isSigNaNRecFN#(expWidth, sigWidth) isSigNaN_a(a, isSigNaNA);
    wire isNaNB, isInfB, isZeroB, signB;
    wire signed [(expWidth + 1):0] sExpB;
    wire [sigWidth:0] sigB;
    recFNToRawFN#(expWidth, sigWidth)
        recFNToRawFN_b(b, isNaNB, isInfB, isZeroB, signB, sExpB, sigB);
    wire isSigNaNB;
    isSigNaNRecFN#(expWidth, sigWidth) isSigNaN_b(b, isSigNaNB);
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    wire notSigNaN_invalidExc = (isInfA && isZeroB) || (isZeroA && isInfB);
    wire notNaN_isInfOut = isInfA || isInfB;
    wire notNaN_isZeroOut = isZeroA || isZeroB;
    wire notNaN_signOut = signA ^ signB;
    wire signed [(expWidth + 1):0] common_sExpOut =
        sExpA + sExpB - (1<<expWidth);
    wire [(sigWidth*2 - 1):0] sigProd = sigA * sigB;
    wire [(sigWidth + 2):0] common_sigOut =
        {sigProd[(sigWidth*2 - 1):(sigWidth - 2)], |sigProd[(sigWidth - 3):0]};
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    assign invalidExc = isSigNaNA || isSigNaNB || notSigNaN_invalidExc;
    assign out_isInf = notNaN_isInfOut;
    assign out_isZero = notNaN_isZeroOut;
    assign out_sExp = common_sExpOut;
`ifdef HardFloat_propagateNaNPayloads
    assign out_isNaN = isNaNA || isNaNB || notSigNaN_invalidExc;
    wire signNaN;
    wire [(sigWidth - 2):0] fractNaN;
    propagateFloatNaN_mul#(sigWidth)
        propagateNaN(
            control,
            isNaNA,
            signA,
            sigA[(sigWidth - 2):0],
            isNaNB,
            signB,
            sigB[(sigWidth - 2):0],
            signNaN,
            fractNaN
        );
    assign out_sign = out_isNaN ? signNaN : notNaN_signOut;
    assign out_sig = out_isNaN ? {1'b1, fractNaN, 2'b00} : common_sigOut;
`else
    assign out_isNaN = isNaNA || isNaNB;
    assign out_sign = notNaN_signOut;
    assign out_sig = common_sigOut;
`endif

endmodule

/*----------------------------------------------------------------------------
*----------------------------------------------------------------------------*/

module
    mulRecFN#(parameter expWidth = 3, parameter sigWidth = 3
    ) (
        input [(`floatControlWidth - 1):0] control,
        input [(expWidth + sigWidth):0] a,
        input [(expWidth + sigWidth):0] b,
        input [2:0] roundingMode,
        output [(expWidth + sigWidth):0] out,
        output [4:0] exceptionFlags
    );

    wire invalidExc, out_isNaN, out_isInf, out_isZero, out_sign;
    wire signed [(expWidth + 1):0] out_sExp;
    wire [(sigWidth + 2):0] out_sig;
    mulRecFNToRaw#(expWidth, sigWidth)
        mulRecFNToRaw(
            control,
            a,
            b,
            invalidExc,
            out_isNaN,
            out_isInf,
            out_isZero,
            out_sign,
            out_sExp,
            out_sig
        );
    roundRawFNToRecFN#(expWidth, sigWidth, 0)
        roundRawOut(
            control,
            invalidExc,
            1'b0,
            out_isNaN,
            out_isInf,
            out_isZero,
            out_sign,
            out_sExp,
            out_sig,
            roundingMode,
            out,
            exceptionFlags
        );

endmodule

`endif /* __HARDFLOAT_MULRECFN_V__ */