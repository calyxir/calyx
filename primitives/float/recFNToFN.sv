`ifndef __HARDFLOAT_RECFNTOFN_V__
`define __HARDFLOAT_RECFNTOFN_V__

`include "primitives/float/HardFloat_rawFN.sv"

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

/* ============= Added section to include some files ================== */
// verilator lint_off MODDUP
//`include "includeFile.v"
// verilator lint_on MODDUP
/* ============================================================== */

/*----------------------------------------------------------------------------
*----------------------------------------------------------------------------*/

module
  recFNToFN#(
    parameter expWidth = 3, 
    parameter sigWidth = 3,
    parameter inputWidth = 7,
    parameter outputWidth = 6
) (
    input [inputWidth-1:0] in_,
    output [outputWidth-1:0] out
  );
`include "primitives/float/HardFloat_localFuncs.vi"

  /*------------------------------------------------------------------------
  *------------------------------------------------------------------------*/
  localparam [expWidth:0] minNormExp = (1<<(expWidth - 1)) + 2;
  localparam normDistWidth = clog2(sigWidth);
  /*------------------------------------------------------------------------
  *------------------------------------------------------------------------*/
  wire isNaN, isInf, isZero, sign;
  wire signed [(expWidth + 1):0] sExp;
  wire [sigWidth:0] sig;
  recFNToRawFN#(expWidth, sigWidth)
      recFNToRawFN(in_, isNaN, isInf, isZero, sign, sExp, sig);
  wire isSubnormal = (sExp < minNormExp);
  /*------------------------------------------------------------------------
  *------------------------------------------------------------------------*/
  wire [(normDistWidth - 1):0] denormShiftDist = minNormExp - 1 - sExp;
  wire [(expWidth - 1):0] expOut =
      (isSubnormal ? 0 : sExp - minNormExp + 1)
          | (isNaN || isInf ? {expWidth{1'b1}} : 0);
  wire [(sigWidth - 2):0] fractOut =
      isSubnormal ? (sig>>1)>>denormShiftDist : isInf ? 0 : sig;
  assign out = {sign, expOut, fractOut};

endmodule

`endif /* __HARDFLOAT_RECFNTOFN_V__ */
