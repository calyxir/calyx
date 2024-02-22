`ifndef __HARDFLOAT_RAWFN_VI__
`define __HARDFLOAT_RAWFN_VI__

`include "primitives/float/source/HardFloat_primitives.v"
`include "primitives/float/source/HardFloat_specialize.vi"
`include "primitives/float/source/HardFloat_consts.vi"
`include "primitives/float/source/RISCV/HardFloat_specialize.vi"

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

/* ===================== Adding some extra include files ================ */
/*`include "RISCV/HardFloat_specialize.vi"
/* ====================================================================== */

/*----------------------------------------------------------------------------
*----------------------------------------------------------------------------*/

module
    recFNToRawFN#(parameter expWidth = 3, parameter sigWidth = 3) (
        input [(expWidth + sigWidth):0] in,
        output isNaN,
        output isInf,
        output isZero,
        output sign,
        output signed [(expWidth + 1):0] sExp,
        output [sigWidth:0] sig
    );

    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    wire [expWidth:0] exp;
    wire [(sigWidth - 2):0] fract;
    assign {sign, exp, fract} = in;
    wire isSpecial = (exp>>(expWidth - 1) == 'b11);
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    assign isNaN = isSpecial &&  exp[expWidth - 2];
    assign isInf = isSpecial && !exp[expWidth - 2];
    assign isZero = (exp>>(expWidth - 2) == 'b000);
    assign sExp = exp;
    assign sig = {1'b0, !isZero, fract};

endmodule

/*----------------------------------------------------------------------------
*----------------------------------------------------------------------------*/

module
    roundAnyRawFNToRecFN#(
        parameter inExpWidth = 3,
        parameter inSigWidth = 3,
        parameter outExpWidth = 3,
        parameter outSigWidth = 3,
        parameter options = 0
    ) (
        input [(`floatControlWidth - 1):0] control,
        input invalidExc,     // overrides 'infiniteExc' and 'in_*' inputs
        input infiniteExc,    // overrides 'in_*' inputs except 'in_sign'
        input in_isNaN,
        input in_isInf,
        input in_isZero,
        input in_sign,
        input signed [(inExpWidth + 1):0] in_sExp,   // limited range allowed
        input [inSigWidth:0] in_sig,
        input [2:0] roundingMode,
        output [(outExpWidth + outSigWidth):0] out,
        output [4:0] exceptionFlags
    );

    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    localparam sigMSBitAlwaysZero =
        ((options & `flRoundOpt_sigMSBitAlwaysZero) != 0);
    localparam effectiveInSigWidth =
        sigMSBitAlwaysZero ? inSigWidth : inSigWidth + 1;
    localparam neverUnderflows =
        ((options
              & (`flRoundOpt_neverUnderflows
                     | `flRoundOpt_subnormsAlwaysExact))
             != 0)
            || (inExpWidth < outExpWidth);
    localparam neverOverflows =
        ((options & `flRoundOpt_neverOverflows) != 0)
            || (inExpWidth < outExpWidth);
    localparam adjustedExpWidth =
          (inExpWidth < outExpWidth) ? outExpWidth + 1
        : (inExpWidth == outExpWidth) ? inExpWidth + 2
        : inExpWidth + 3;
    localparam outNaNExp = 7<<(outExpWidth - 2);
    localparam outInfExp = 6<<(outExpWidth - 2);
    localparam outMaxFiniteExp = outInfExp - 1;
    localparam outMinNormExp = (1<<(outExpWidth - 1)) + 2;
    localparam outMinNonzeroExp = outMinNormExp - outSigWidth + 1;
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    wire roundingMode_near_even   = (roundingMode == `round_near_even);
    wire roundingMode_minMag      = (roundingMode == `round_minMag);
    wire roundingMode_min         = (roundingMode == `round_min);
    wire roundingMode_max         = (roundingMode == `round_max);
    wire roundingMode_near_maxMag = (roundingMode == `round_near_maxMag);
    wire roundingMode_odd         = (roundingMode == `round_odd);
    wire roundMagUp =
        (roundingMode_min && in_sign) || (roundingMode_max && !in_sign);
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    wire isNaNOut = invalidExc || (!infiniteExc && in_isNaN);
`ifdef HardFloat_propagateNaNPayloads
    wire propagateNaNPayload = isNaNOut;
`else
    wire propagateNaNPayload = 0;
`endif
    wire signed [(adjustedExpWidth - 1):0] sAdjustedExp =
        in_sExp + ((1<<outExpWidth) - (1<<inExpWidth));
    wire [(outSigWidth + 2):0] adjustedSig;
    generate
        if (inSigWidth <= outSigWidth + 2) begin
            assign adjustedSig = in_sig<<(outSigWidth - inSigWidth + 2);
        end else begin
            assign adjustedSig =
                {in_sig[inSigWidth:(inSigWidth - outSigWidth - 1)],
                 |in_sig[(inSigWidth - outSigWidth - 2):0]};
        end
    endgenerate
    wire doShiftSigDown1 =
        sigMSBitAlwaysZero ? 0 : adjustedSig[outSigWidth + 2];
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    wire [outExpWidth:0] common_expOut;
    wire [(outSigWidth - 2):0] common_fractOut;
    wire common_overflow, common_totalUnderflow, common_underflow;
    wire common_inexact;
    generate
        if (
            neverOverflows && neverUnderflows
                && (effectiveInSigWidth <= outSigWidth)
        ) begin
            /*----------------------------------------------------------------
            *----------------------------------------------------------------*/
            assign common_expOut = sAdjustedExp + doShiftSigDown1;
            assign common_fractOut =
                doShiftSigDown1 ? adjustedSig>>3 : adjustedSig>>2;
            assign common_overflow       = 0;
            assign common_totalUnderflow = 0;
            assign common_underflow      = 0;
            assign common_inexact        = 0;
        end else begin
            /*----------------------------------------------------------------
            *----------------------------------------------------------------*/
            wire [(outSigWidth + 2):0] roundMask;
            if (neverUnderflows) begin
                assign roundMask = {doShiftSigDown1, 2'b11};
            end else begin
                wire [outSigWidth:0] roundMask_main;
                lowMaskLoHi#(
                    outExpWidth + 1,
                    outMinNormExp - outSigWidth - 1,
                    outMinNormExp
                ) lowMask_roundMask(
                        sAdjustedExp[outExpWidth:0]
                            | (propagateNaNPayload ? 1'b1<<outExpWidth : 1'b0),
                        roundMask_main
                    );
                assign roundMask = {roundMask_main | doShiftSigDown1, 2'b11};
            end
            wire [(outSigWidth + 2):0] shiftedRoundMask = roundMask>>1;
            wire [(outSigWidth + 2):0] roundPosMask =
                ~shiftedRoundMask & roundMask;
            wire roundPosBit =
                (|(adjustedSig[(outSigWidth + 2):3]
                       & roundPosMask[(outSigWidth + 2):3]))
                    || ((|(adjustedSig[2:0] & roundPosMask[2:0]))
                            && !propagateNaNPayload);
            wire anyRoundExtra =
                (|(adjustedSig[(outSigWidth + 2):3]
                       & shiftedRoundMask[(outSigWidth + 2):3]))
                    || ((|(adjustedSig[2:0] & shiftedRoundMask[2:0]))
                            && !propagateNaNPayload);
            wire anyRound = roundPosBit || anyRoundExtra;
            /*----------------------------------------------------------------
            *----------------------------------------------------------------*/
            wire roundIncr =
                ((roundingMode_near_even || roundingMode_near_maxMag)
                     && roundPosBit)
                    || (roundMagUp && anyRound);
            wire [(outSigWidth + 1):0] roundedSig =
                roundIncr
                    ? (((adjustedSig | roundMask)>>2) + 1)
                          & ~(roundingMode_near_even && roundPosBit
                                  && !anyRoundExtra
                                  ? roundMask>>1 : 0)
                    : (adjustedSig & ~roundMask)>>2
                          | (roundingMode_odd && anyRound
                                 ? roundPosMask>>1 : 0);
            wire signed [adjustedExpWidth:0] sExtAdjustedExp = sAdjustedExp;
            wire signed [adjustedExpWidth:0] sRoundedExp =
                sExtAdjustedExp + (roundedSig>>outSigWidth);
            /*----------------------------------------------------------------
            *----------------------------------------------------------------*/
            assign common_expOut = sRoundedExp;
            assign common_fractOut =
                doShiftSigDown1 ? roundedSig>>1 : roundedSig;
            assign common_overflow =
                neverOverflows ? 0 : (sRoundedExp>>>(outExpWidth - 1) >= 3);
            assign common_totalUnderflow =
                neverUnderflows ? 0 : (sRoundedExp < outMinNonzeroExp);
            /*----------------------------------------------------------------
            *----------------------------------------------------------------*/
            wire unboundedRange_roundPosBit =
                doShiftSigDown1 ? adjustedSig[2] : adjustedSig[1];
            wire unboundedRange_anyRound =
                (doShiftSigDown1 && adjustedSig[2]) || (|adjustedSig[1:0]);
            wire unboundedRange_roundIncr =
                ((roundingMode_near_even || roundingMode_near_maxMag)
                     && unboundedRange_roundPosBit)
                    || (roundMagUp && unboundedRange_anyRound);
            wire roundCarry =
                doShiftSigDown1
                    ? roundedSig[outSigWidth + 1] : roundedSig[outSigWidth];
            assign common_underflow =
                neverUnderflows ? 0
                    : common_totalUnderflow
                          || (anyRound && (sAdjustedExp>>>outExpWidth <= 0)
                                  && (doShiftSigDown1
                                          ? roundMask[3] : roundMask[2])
                                  && !(((control
                                           & `flControl_tininessAfterRounding)
                                            != 0)
                                           && !(doShiftSigDown1 ? roundMask[4]
                                                    : roundMask[3])
                                           && roundCarry && roundPosBit
                                           && unboundedRange_roundIncr));
            /*----------------------------------------------------------------
            *----------------------------------------------------------------*/
            assign common_inexact = common_totalUnderflow || anyRound;
        end
    endgenerate
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    wire notNaN_isSpecialInfOut = infiniteExc || in_isInf;
    wire commonCase = !isNaNOut && !notNaN_isSpecialInfOut && !in_isZero;
    wire overflow  = commonCase && common_overflow;
    wire underflow = commonCase && common_underflow;
    wire inexact = overflow || (commonCase && common_inexact);
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    wire overflow_roundMagUp =
        roundingMode_near_even || roundingMode_near_maxMag || roundMagUp;
    wire pegMinNonzeroMagOut =
        commonCase && common_totalUnderflow
            && (roundMagUp || roundingMode_odd);
    wire pegMaxFiniteMagOut = overflow && !overflow_roundMagUp;
    wire notNaN_isInfOut =
        notNaN_isSpecialInfOut || (overflow && overflow_roundMagUp);
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
`ifdef HardFloat_propagateNaNPayloads
    wire signOut = in_sign;
`else
    wire signOut = isNaNOut ? `HardFloat_signDefaultNaN : in_sign;
`endif
    wire [outExpWidth:0] expOut =
        (common_expOut
             & ~(in_isZero || common_totalUnderflow ? 7<<(outExpWidth - 2) : 0)
             & ~(pegMinNonzeroMagOut               ? ~outMinNonzeroExp    : 0)
             & ~(pegMaxFiniteMagOut                ? 1<<(outExpWidth - 1) : 0)
             & ~(notNaN_isInfOut                   ? 1<<(outExpWidth - 2) : 0))
            | (pegMinNonzeroMagOut ? outMinNonzeroExp : 0)
            | (pegMaxFiniteMagOut  ? outMaxFiniteExp  : 0)
            | (notNaN_isInfOut     ? outInfExp        : 0)
            | (isNaNOut            ? outNaNExp        : 0);
`ifdef HardFloat_propagateNaNPayloads
    wire [(outSigWidth - 2):0] fractOut =
        {isNaNOut
             || (!in_isZero && !common_totalUnderflow
                     && common_fractOut[outSigWidth - 2]),
         isNaNOut || (!in_isZero && !common_totalUnderflow)
             ? common_fractOut[(outSigWidth - 3):0] : 1'b0}
            | {(outSigWidth - 1){pegMaxFiniteMagOut}};
`else
    wire [(outSigWidth - 2):0] fractOut =
          (isNaNOut ? `HardFloat_fractDefaultNaN(outSigWidth) : 0)
        | (!in_isZero && !common_totalUnderflow
               ? common_fractOut & `HardFloat_fractDefaultNaN(outSigWidth) : 0)
        | (!isNaNOut && !in_isZero && !common_totalUnderflow
               ? common_fractOut & ~`HardFloat_fractDefaultNaN(outSigWidth)
               : 0)
        | {(outSigWidth - 1){pegMaxFiniteMagOut}};
`endif
    assign out = {signOut, expOut, fractOut};
    assign exceptionFlags =
        {invalidExc, infiniteExc, overflow, underflow, inexact};

endmodule

/*----------------------------------------------------------------------------
*----------------------------------------------------------------------------*/

module
    roundRawFNToRecFN#(
        parameter expWidth = 3,
        parameter sigWidth = 3,
        parameter options = 0
    ) (
        input [(`floatControlWidth - 1):0] control,
        input invalidExc,     // overrides 'infiniteExc' and 'in_*' inputs
        input infiniteExc,    // overrides 'in_*' inputs except 'in_sign'
        input in_isNaN,
        input in_isInf,
        input in_isZero,
        input in_sign,
        input signed [(expWidth + 1):0] in_sExp,   // limited range allowed
        input [(sigWidth + 2):0] in_sig,
        input [2:0] roundingMode,
        output [(expWidth + sigWidth):0] out,
        output [4:0] exceptionFlags
    );

    roundAnyRawFNToRecFN#(expWidth, sigWidth + 2, expWidth, sigWidth, options)
        roundAnyRawFNToRecFN(
            control,
            invalidExc,
            infiniteExc,
            in_isNaN,
            in_isInf,
            in_isZero,
            in_sign,
            in_sExp,
            in_sig,
            roundingMode,
            out,
            exceptionFlags
        );

endmodule

`endif /* __HARDFLOAT_RAWFN_VI__ */
