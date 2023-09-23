module MulFullRawFN(
  input         io_a_isNaN,
  input         io_a_isInf,
  input         io_a_isZero,
  input         io_a_sign,
  input  [9:0]  io_a_sExp,
  input  [24:0] io_a_sig,
  input         io_b_isNaN,
  input         io_b_isInf,
  input         io_b_isZero,
  input         io_b_sign,
  input  [9:0]  io_b_sExp,
  input  [24:0] io_b_sig,
  output        io_invalidExc,
  output        io_rawOut_isNaN,
  output        io_rawOut_isInf,
  output        io_rawOut_isZero,
  output        io_rawOut_sign,
  output [9:0]  io_rawOut_sExp,
  output [47:0] io_rawOut_sig
);
  wire  notSigNaN_invalidExc = io_a_isInf & io_b_isZero | io_a_isZero & io_b_isInf; // @[MulRecFN.scala 58:60]
  wire [9:0] _T_4 = $signed(io_a_sExp) + $signed(io_b_sExp); // @[MulRecFN.scala 62:36]
  wire [49:0] _T_7 = io_a_sig * io_b_sig; // @[MulRecFN.scala 63:35]
  wire  _T_10 = io_a_isNaN & ~io_a_sig[22]; // @[common.scala 84:46]
  wire  _T_13 = io_b_isNaN & ~io_b_sig[22]; // @[common.scala 84:46]
  assign io_invalidExc = _T_10 | _T_13 | notSigNaN_invalidExc; // @[MulRecFN.scala 66:71]
  assign io_rawOut_isNaN = io_a_isNaN | io_b_isNaN; // @[MulRecFN.scala 70:35]
  assign io_rawOut_isInf = io_a_isInf | io_b_isInf; // @[MulRecFN.scala 59:38]
  assign io_rawOut_isZero = io_a_isZero | io_b_isZero; // @[MulRecFN.scala 60:40]
  assign io_rawOut_sign = io_a_sign ^ io_b_sign; // @[MulRecFN.scala 61:36]
  assign io_rawOut_sExp = $signed(_T_4) - 10'sh100; // @[MulRecFN.scala 62:48]
  assign io_rawOut_sig = _T_7[47:0]; // @[MulRecFN.scala 63:46]
endmodule
module MulRawFN(
  input         io_a_isNaN,
  input         io_a_isInf,
  input         io_a_isZero,
  input         io_a_sign,
  input  [9:0]  io_a_sExp,
  input  [24:0] io_a_sig,
  input         io_b_isNaN,
  input         io_b_isInf,
  input         io_b_isZero,
  input         io_b_sign,
  input  [9:0]  io_b_sExp,
  input  [24:0] io_b_sig,
  output        io_invalidExc,
  output        io_rawOut_isNaN,
  output        io_rawOut_isInf,
  output        io_rawOut_isZero,
  output        io_rawOut_sign,
  output [9:0]  io_rawOut_sExp,
  output [26:0] io_rawOut_sig
);
  wire  mulFullRaw_io_a_isNaN; // @[MulRecFN.scala 84:28]
  wire  mulFullRaw_io_a_isInf; // @[MulRecFN.scala 84:28]
  wire  mulFullRaw_io_a_isZero; // @[MulRecFN.scala 84:28]
  wire  mulFullRaw_io_a_sign; // @[MulRecFN.scala 84:28]
  wire [9:0] mulFullRaw_io_a_sExp; // @[MulRecFN.scala 84:28]
  wire [24:0] mulFullRaw_io_a_sig; // @[MulRecFN.scala 84:28]
  wire  mulFullRaw_io_b_isNaN; // @[MulRecFN.scala 84:28]
  wire  mulFullRaw_io_b_isInf; // @[MulRecFN.scala 84:28]
  wire  mulFullRaw_io_b_isZero; // @[MulRecFN.scala 84:28]
  wire  mulFullRaw_io_b_sign; // @[MulRecFN.scala 84:28]
  wire [9:0] mulFullRaw_io_b_sExp; // @[MulRecFN.scala 84:28]
  wire [24:0] mulFullRaw_io_b_sig; // @[MulRecFN.scala 84:28]
  wire  mulFullRaw_io_invalidExc; // @[MulRecFN.scala 84:28]
  wire  mulFullRaw_io_rawOut_isNaN; // @[MulRecFN.scala 84:28]
  wire  mulFullRaw_io_rawOut_isInf; // @[MulRecFN.scala 84:28]
  wire  mulFullRaw_io_rawOut_isZero; // @[MulRecFN.scala 84:28]
  wire  mulFullRaw_io_rawOut_sign; // @[MulRecFN.scala 84:28]
  wire [9:0] mulFullRaw_io_rawOut_sExp; // @[MulRecFN.scala 84:28]
  wire [47:0] mulFullRaw_io_rawOut_sig; // @[MulRecFN.scala 84:28]
  wire [25:0] hi = mulFullRaw_io_rawOut_sig[47:22]; // @[MulRecFN.scala 93:15]
  wire  lo = |mulFullRaw_io_rawOut_sig[21:0]; // @[MulRecFN.scala 93:55]
  MulFullRawFN mulFullRaw ( // @[MulRecFN.scala 84:28]
    .io_a_isNaN(mulFullRaw_io_a_isNaN),
    .io_a_isInf(mulFullRaw_io_a_isInf),
    .io_a_isZero(mulFullRaw_io_a_isZero),
    .io_a_sign(mulFullRaw_io_a_sign),
    .io_a_sExp(mulFullRaw_io_a_sExp),
    .io_a_sig(mulFullRaw_io_a_sig),
    .io_b_isNaN(mulFullRaw_io_b_isNaN),
    .io_b_isInf(mulFullRaw_io_b_isInf),
    .io_b_isZero(mulFullRaw_io_b_isZero),
    .io_b_sign(mulFullRaw_io_b_sign),
    .io_b_sExp(mulFullRaw_io_b_sExp),
    .io_b_sig(mulFullRaw_io_b_sig),
    .io_invalidExc(mulFullRaw_io_invalidExc),
    .io_rawOut_isNaN(mulFullRaw_io_rawOut_isNaN),
    .io_rawOut_isInf(mulFullRaw_io_rawOut_isInf),
    .io_rawOut_isZero(mulFullRaw_io_rawOut_isZero),
    .io_rawOut_sign(mulFullRaw_io_rawOut_sign),
    .io_rawOut_sExp(mulFullRaw_io_rawOut_sExp),
    .io_rawOut_sig(mulFullRaw_io_rawOut_sig)
  );
  assign io_invalidExc = mulFullRaw_io_invalidExc; // @[MulRecFN.scala 89:19]
  assign io_rawOut_isNaN = mulFullRaw_io_rawOut_isNaN; // @[MulRecFN.scala 90:15]
  assign io_rawOut_isInf = mulFullRaw_io_rawOut_isInf; // @[MulRecFN.scala 90:15]
  assign io_rawOut_isZero = mulFullRaw_io_rawOut_isZero; // @[MulRecFN.scala 90:15]
  assign io_rawOut_sign = mulFullRaw_io_rawOut_sign; // @[MulRecFN.scala 90:15]
  assign io_rawOut_sExp = mulFullRaw_io_rawOut_sExp; // @[MulRecFN.scala 90:15]
  assign io_rawOut_sig = {hi,lo}; // @[Cat.scala 30:58]
  assign mulFullRaw_io_a_isNaN = io_a_isNaN; // @[MulRecFN.scala 86:21]
  assign mulFullRaw_io_a_isInf = io_a_isInf; // @[MulRecFN.scala 86:21]
  assign mulFullRaw_io_a_isZero = io_a_isZero; // @[MulRecFN.scala 86:21]
  assign mulFullRaw_io_a_sign = io_a_sign; // @[MulRecFN.scala 86:21]
  assign mulFullRaw_io_a_sExp = io_a_sExp; // @[MulRecFN.scala 86:21]
  assign mulFullRaw_io_a_sig = io_a_sig; // @[MulRecFN.scala 86:21]
  assign mulFullRaw_io_b_isNaN = io_b_isNaN; // @[MulRecFN.scala 87:21]
  assign mulFullRaw_io_b_isInf = io_b_isInf; // @[MulRecFN.scala 87:21]
  assign mulFullRaw_io_b_isZero = io_b_isZero; // @[MulRecFN.scala 87:21]
  assign mulFullRaw_io_b_sign = io_b_sign; // @[MulRecFN.scala 87:21]
  assign mulFullRaw_io_b_sExp = io_b_sExp; // @[MulRecFN.scala 87:21]
  assign mulFullRaw_io_b_sig = io_b_sig; // @[MulRecFN.scala 87:21]
endmodule
module RoundAnyRawFNToRecFN(
  input         io_invalidExc,
  input         io_in_isNaN,
  input         io_in_isInf,
  input         io_in_isZero,
  input         io_in_sign,
  input  [9:0]  io_in_sExp,
  input  [26:0] io_in_sig,
  output [32:0] io_out
);
  wire  doShiftSigDown1 = io_in_sig[26]; // @[RoundAnyRawFNToRecFN.scala 118:61]
  wire [8:0] _T_4 = ~io_in_sExp[8:0]; // @[primitives.scala 51:21]
  wire [64:0] _T_11 = 65'sh10000000000000000 >>> _T_4[5:0]; // @[primitives.scala 77:58]
  wire [15:0] _T_17 = {{8'd0}, _T_11[57:50]}; // @[Bitwise.scala 103:31]
  wire [15:0] _T_19 = {_T_11[49:42], 8'h0}; // @[Bitwise.scala 103:65]
  wire [15:0] _T_21 = _T_19 & 16'hff00; // @[Bitwise.scala 103:75]
  wire [15:0] _T_22 = _T_17 | _T_21; // @[Bitwise.scala 103:39]
  wire [15:0] _GEN_0 = {{4'd0}, _T_22[15:4]}; // @[Bitwise.scala 103:31]
  wire [15:0] _T_27 = _GEN_0 & 16'hf0f; // @[Bitwise.scala 103:31]
  wire [15:0] _T_29 = {_T_22[11:0], 4'h0}; // @[Bitwise.scala 103:65]
  wire [15:0] _T_31 = _T_29 & 16'hf0f0; // @[Bitwise.scala 103:75]
  wire [15:0] _T_32 = _T_27 | _T_31; // @[Bitwise.scala 103:39]
  wire [15:0] _GEN_1 = {{2'd0}, _T_32[15:2]}; // @[Bitwise.scala 103:31]
  wire [15:0] _T_37 = _GEN_1 & 16'h3333; // @[Bitwise.scala 103:31]
  wire [15:0] _T_39 = {_T_32[13:0], 2'h0}; // @[Bitwise.scala 103:65]
  wire [15:0] _T_41 = _T_39 & 16'hcccc; // @[Bitwise.scala 103:75]
  wire [15:0] _T_42 = _T_37 | _T_41; // @[Bitwise.scala 103:39]
  wire [15:0] _GEN_2 = {{1'd0}, _T_42[15:1]}; // @[Bitwise.scala 103:31]
  wire [15:0] _T_47 = _GEN_2 & 16'h5555; // @[Bitwise.scala 103:31]
  wire [15:0] _T_49 = {_T_42[14:0], 1'h0}; // @[Bitwise.scala 103:65]
  wire [15:0] _T_51 = _T_49 & 16'haaaa; // @[Bitwise.scala 103:75]
  wire [15:0] hi = _T_47 | _T_51; // @[Bitwise.scala 103:39]
  wire  hi_1 = _T_11[58]; // @[Bitwise.scala 109:18]
  wire  lo = _T_11[59]; // @[Bitwise.scala 109:44]
  wire  hi_3 = _T_11[60]; // @[Bitwise.scala 109:18]
  wire  lo_1 = _T_11[61]; // @[Bitwise.scala 109:44]
  wire  hi_5 = _T_11[62]; // @[Bitwise.scala 109:18]
  wire  lo_3 = _T_11[63]; // @[Bitwise.scala 109:44]
  wire [21:0] _T_57 = {hi,hi_1,lo,hi_3,lo_1,hi_5,lo_3}; // @[Cat.scala 30:58]
  wire [21:0] _T_58 = ~_T_57; // @[primitives.scala 74:36]
  wire [21:0] _T_59 = _T_4[6] ? 22'h0 : _T_58; // @[primitives.scala 74:21]
  wire [21:0] hi_6 = ~_T_59; // @[primitives.scala 74:17]
  wire [24:0] _T_60 = {hi_6,3'h7}; // @[Cat.scala 30:58]
  wire  hi_7 = _T_11[0]; // @[Bitwise.scala 109:18]
  wire  lo_6 = _T_11[1]; // @[Bitwise.scala 109:44]
  wire  lo_7 = _T_11[2]; // @[Bitwise.scala 109:44]
  wire [2:0] _T_66 = {hi_7,lo_6,lo_7}; // @[Cat.scala 30:58]
  wire [2:0] _T_67 = _T_4[6] ? _T_66 : 3'h0; // @[primitives.scala 61:24]
  wire [24:0] _T_68 = _T_4[7] ? _T_60 : {{22'd0}, _T_67}; // @[primitives.scala 66:24]
  wire [24:0] _T_69 = _T_4[8] ? _T_68 : 25'h0; // @[primitives.scala 61:24]
  wire [24:0] _GEN_3 = {{24'd0}, doShiftSigDown1}; // @[RoundAnyRawFNToRecFN.scala 157:23]
  wire [24:0] hi_9 = _T_69 | _GEN_3; // @[RoundAnyRawFNToRecFN.scala 157:23]
  wire [26:0] _T_70 = {hi_9,2'h3}; // @[Cat.scala 30:58]
  wire [25:0] lo_8 = _T_70[26:1]; // @[RoundAnyRawFNToRecFN.scala 160:57]
  wire [26:0] _T_71 = {1'h0,lo_8}; // @[Cat.scala 30:58]
  wire [26:0] _T_72 = ~_T_71; // @[RoundAnyRawFNToRecFN.scala 161:28]
  wire [26:0] _T_73 = _T_72 & _T_70; // @[RoundAnyRawFNToRecFN.scala 161:46]
  wire [26:0] _T_74 = io_in_sig & _T_73; // @[RoundAnyRawFNToRecFN.scala 162:40]
  wire  _T_75 = |_T_74; // @[RoundAnyRawFNToRecFN.scala 162:56]
  wire [26:0] _T_76 = io_in_sig & _T_71; // @[RoundAnyRawFNToRecFN.scala 163:42]
  wire  _T_77 = |_T_76; // @[RoundAnyRawFNToRecFN.scala 163:62]
  wire [26:0] _T_83 = io_in_sig | _T_70; // @[RoundAnyRawFNToRecFN.scala 172:32]
  wire [25:0] _T_85 = _T_83[26:2] + 25'h1; // @[RoundAnyRawFNToRecFN.scala 172:49]
  wire  _T_87 = ~_T_77; // @[RoundAnyRawFNToRecFN.scala 174:30]
  wire [25:0] _T_90 = _T_75 & _T_87 ? lo_8 : 26'h0; // @[RoundAnyRawFNToRecFN.scala 173:25]
  wire [25:0] _T_91 = ~_T_90; // @[RoundAnyRawFNToRecFN.scala 173:21]
  wire [25:0] _T_92 = _T_85 & _T_91; // @[RoundAnyRawFNToRecFN.scala 172:61]
  wire [26:0] _T_93 = ~_T_70; // @[RoundAnyRawFNToRecFN.scala 178:32]
  wire [26:0] _T_94 = io_in_sig & _T_93; // @[RoundAnyRawFNToRecFN.scala 178:30]
  wire [25:0] _T_99 = {{1'd0}, _T_94[26:2]}; // @[RoundAnyRawFNToRecFN.scala 178:47]
  wire [25:0] _T_100 = _T_75 ? _T_92 : _T_99; // @[RoundAnyRawFNToRecFN.scala 171:16]
  wire [2:0] _T_102 = {1'b0,$signed(_T_100[25:24])}; // @[RoundAnyRawFNToRecFN.scala 183:69]
  wire [9:0] _GEN_4 = {{7{_T_102[2]}},_T_102}; // @[RoundAnyRawFNToRecFN.scala 183:40]
  wire [10:0] _T_103 = $signed(io_in_sExp) + $signed(_GEN_4); // @[RoundAnyRawFNToRecFN.scala 183:40]
  wire [8:0] common_expOut = _T_103[8:0]; // @[RoundAnyRawFNToRecFN.scala 185:37]
  wire [22:0] common_fractOut = doShiftSigDown1 ? _T_100[23:1] : _T_100[22:0]; // @[RoundAnyRawFNToRecFN.scala 187:16]
  wire [3:0] _T_108 = _T_103[10:7]; // @[RoundAnyRawFNToRecFN.scala 194:30]
  wire  common_overflow = $signed(_T_108) >= 4'sh3; // @[RoundAnyRawFNToRecFN.scala 194:50]
  wire  common_totalUnderflow = $signed(_T_103) < 11'sh6b; // @[RoundAnyRawFNToRecFN.scala 198:31]
  wire  isNaNOut = io_invalidExc | io_in_isNaN; // @[RoundAnyRawFNToRecFN.scala 233:34]
  wire  commonCase = ~isNaNOut & ~io_in_isInf & ~io_in_isZero; // @[RoundAnyRawFNToRecFN.scala 235:61]
  wire  overflow = commonCase & common_overflow; // @[RoundAnyRawFNToRecFN.scala 236:32]
  wire  notNaN_isInfOut = io_in_isInf | overflow; // @[RoundAnyRawFNToRecFN.scala 246:32]
  wire  signOut = isNaNOut ? 1'h0 : io_in_sign; // @[RoundAnyRawFNToRecFN.scala 248:22]
  wire [8:0] _T_157 = io_in_isZero | common_totalUnderflow ? 9'h1c0 : 9'h0; // @[RoundAnyRawFNToRecFN.scala 251:18]
  wire [8:0] _T_158 = ~_T_157; // @[RoundAnyRawFNToRecFN.scala 251:14]
  wire [8:0] _T_159 = common_expOut & _T_158; // @[RoundAnyRawFNToRecFN.scala 250:24]
  wire [8:0] _T_167 = notNaN_isInfOut ? 9'h40 : 9'h0; // @[RoundAnyRawFNToRecFN.scala 263:18]
  wire [8:0] _T_168 = ~_T_167; // @[RoundAnyRawFNToRecFN.scala 263:14]
  wire [8:0] _T_169 = _T_159 & _T_168; // @[RoundAnyRawFNToRecFN.scala 262:17]
  wire [8:0] _T_174 = notNaN_isInfOut ? 9'h180 : 9'h0; // @[RoundAnyRawFNToRecFN.scala 275:16]
  wire [8:0] _T_175 = _T_169 | _T_174; // @[RoundAnyRawFNToRecFN.scala 274:15]
  wire [8:0] _T_176 = isNaNOut ? 9'h1c0 : 9'h0; // @[RoundAnyRawFNToRecFN.scala 276:16]
  wire [8:0] expOut = _T_175 | _T_176; // @[RoundAnyRawFNToRecFN.scala 275:77]
  wire [22:0] _T_179 = isNaNOut ? 23'h400000 : 23'h0; // @[RoundAnyRawFNToRecFN.scala 279:16]
  wire [22:0] fractOut = isNaNOut | io_in_isZero | common_totalUnderflow ? _T_179 : common_fractOut; // @[RoundAnyRawFNToRecFN.scala 278:12]
  wire [9:0] hi_10 = {signOut,expOut}; // @[Cat.scala 30:58]
  assign io_out = {hi_10,fractOut}; // @[Cat.scala 30:58]
endmodule
module RoundRawFNToRecFN(
  input         io_invalidExc,
  input         io_in_isNaN,
  input         io_in_isInf,
  input         io_in_isZero,
  input         io_in_sign,
  input  [9:0]  io_in_sExp,
  input  [26:0] io_in_sig,
  output [32:0] io_out
);
  wire  roundAnyRawFNToRecFN_io_invalidExc; // @[RoundAnyRawFNToRecFN.scala 307:15]
  wire  roundAnyRawFNToRecFN_io_in_isNaN; // @[RoundAnyRawFNToRecFN.scala 307:15]
  wire  roundAnyRawFNToRecFN_io_in_isInf; // @[RoundAnyRawFNToRecFN.scala 307:15]
  wire  roundAnyRawFNToRecFN_io_in_isZero; // @[RoundAnyRawFNToRecFN.scala 307:15]
  wire  roundAnyRawFNToRecFN_io_in_sign; // @[RoundAnyRawFNToRecFN.scala 307:15]
  wire [9:0] roundAnyRawFNToRecFN_io_in_sExp; // @[RoundAnyRawFNToRecFN.scala 307:15]
  wire [26:0] roundAnyRawFNToRecFN_io_in_sig; // @[RoundAnyRawFNToRecFN.scala 307:15]
  wire [32:0] roundAnyRawFNToRecFN_io_out; // @[RoundAnyRawFNToRecFN.scala 307:15]
  RoundAnyRawFNToRecFN roundAnyRawFNToRecFN ( // @[RoundAnyRawFNToRecFN.scala 307:15]
    .io_invalidExc(roundAnyRawFNToRecFN_io_invalidExc),
    .io_in_isNaN(roundAnyRawFNToRecFN_io_in_isNaN),
    .io_in_isInf(roundAnyRawFNToRecFN_io_in_isInf),
    .io_in_isZero(roundAnyRawFNToRecFN_io_in_isZero),
    .io_in_sign(roundAnyRawFNToRecFN_io_in_sign),
    .io_in_sExp(roundAnyRawFNToRecFN_io_in_sExp),
    .io_in_sig(roundAnyRawFNToRecFN_io_in_sig),
    .io_out(roundAnyRawFNToRecFN_io_out)
  );
  assign io_out = roundAnyRawFNToRecFN_io_out; // @[RoundAnyRawFNToRecFN.scala 315:23]
  assign roundAnyRawFNToRecFN_io_invalidExc = io_invalidExc; // @[RoundAnyRawFNToRecFN.scala 310:44]
  assign roundAnyRawFNToRecFN_io_in_isNaN = io_in_isNaN; // @[RoundAnyRawFNToRecFN.scala 312:44]
  assign roundAnyRawFNToRecFN_io_in_isInf = io_in_isInf; // @[RoundAnyRawFNToRecFN.scala 312:44]
  assign roundAnyRawFNToRecFN_io_in_isZero = io_in_isZero; // @[RoundAnyRawFNToRecFN.scala 312:44]
  assign roundAnyRawFNToRecFN_io_in_sign = io_in_sign; // @[RoundAnyRawFNToRecFN.scala 312:44]
  assign roundAnyRawFNToRecFN_io_in_sExp = io_in_sExp; // @[RoundAnyRawFNToRecFN.scala 312:44]
  assign roundAnyRawFNToRecFN_io_in_sig = io_in_sig; // @[RoundAnyRawFNToRecFN.scala 312:44]
endmodule
module MulRecFN(
  input  [32:0] io_a,
  input  [32:0] io_b,
  output [32:0] io_out
);
  wire  mulRawFN_io_a_isNaN; // @[MulRecFN.scala 113:26]
  wire  mulRawFN_io_a_isInf; // @[MulRecFN.scala 113:26]
  wire  mulRawFN_io_a_isZero; // @[MulRecFN.scala 113:26]
  wire  mulRawFN_io_a_sign; // @[MulRecFN.scala 113:26]
  wire [9:0] mulRawFN_io_a_sExp; // @[MulRecFN.scala 113:26]
  wire [24:0] mulRawFN_io_a_sig; // @[MulRecFN.scala 113:26]
  wire  mulRawFN_io_b_isNaN; // @[MulRecFN.scala 113:26]
  wire  mulRawFN_io_b_isInf; // @[MulRecFN.scala 113:26]
  wire  mulRawFN_io_b_isZero; // @[MulRecFN.scala 113:26]
  wire  mulRawFN_io_b_sign; // @[MulRecFN.scala 113:26]
  wire [9:0] mulRawFN_io_b_sExp; // @[MulRecFN.scala 113:26]
  wire [24:0] mulRawFN_io_b_sig; // @[MulRecFN.scala 113:26]
  wire  mulRawFN_io_invalidExc; // @[MulRecFN.scala 113:26]
  wire  mulRawFN_io_rawOut_isNaN; // @[MulRecFN.scala 113:26]
  wire  mulRawFN_io_rawOut_isInf; // @[MulRecFN.scala 113:26]
  wire  mulRawFN_io_rawOut_isZero; // @[MulRecFN.scala 113:26]
  wire  mulRawFN_io_rawOut_sign; // @[MulRecFN.scala 113:26]
  wire [9:0] mulRawFN_io_rawOut_sExp; // @[MulRecFN.scala 113:26]
  wire [26:0] mulRawFN_io_rawOut_sig; // @[MulRecFN.scala 113:26]
  wire  roundRawFNToRecFN_io_invalidExc; // @[MulRecFN.scala 121:15]
  wire  roundRawFNToRecFN_io_in_isNaN; // @[MulRecFN.scala 121:15]
  wire  roundRawFNToRecFN_io_in_isInf; // @[MulRecFN.scala 121:15]
  wire  roundRawFNToRecFN_io_in_isZero; // @[MulRecFN.scala 121:15]
  wire  roundRawFNToRecFN_io_in_sign; // @[MulRecFN.scala 121:15]
  wire [9:0] roundRawFNToRecFN_io_in_sExp; // @[MulRecFN.scala 121:15]
  wire [26:0] roundRawFNToRecFN_io_in_sig; // @[MulRecFN.scala 121:15]
  wire [32:0] roundRawFNToRecFN_io_out; // @[MulRecFN.scala 121:15]
  wire  _T_2 = io_a[31:29] == 3'h0; // @[rawFloatFromRecFN.scala 51:54]
  wire  _T_4 = io_a[31:30] == 2'h3; // @[rawFloatFromRecFN.scala 52:54]
  wire  hi_lo = ~_T_2; // @[rawFloatFromRecFN.scala 60:39]
  wire [22:0] lo = io_a[22:0]; // @[rawFloatFromRecFN.scala 60:51]
  wire [1:0] hi = {1'h0,hi_lo}; // @[Cat.scala 30:58]
  wire  _T_15 = io_b[31:29] == 3'h0; // @[rawFloatFromRecFN.scala 51:54]
  wire  _T_17 = io_b[31:30] == 2'h3; // @[rawFloatFromRecFN.scala 52:54]
  wire  hi_lo_1 = ~_T_15; // @[rawFloatFromRecFN.scala 60:39]
  wire [22:0] lo_1 = io_b[22:0]; // @[rawFloatFromRecFN.scala 60:51]
  wire [1:0] hi_1 = {1'h0,hi_lo_1}; // @[Cat.scala 30:58]
  MulRawFN mulRawFN ( // @[MulRecFN.scala 113:26]
    .io_a_isNaN(mulRawFN_io_a_isNaN),
    .io_a_isInf(mulRawFN_io_a_isInf),
    .io_a_isZero(mulRawFN_io_a_isZero),
    .io_a_sign(mulRawFN_io_a_sign),
    .io_a_sExp(mulRawFN_io_a_sExp),
    .io_a_sig(mulRawFN_io_a_sig),
    .io_b_isNaN(mulRawFN_io_b_isNaN),
    .io_b_isInf(mulRawFN_io_b_isInf),
    .io_b_isZero(mulRawFN_io_b_isZero),
    .io_b_sign(mulRawFN_io_b_sign),
    .io_b_sExp(mulRawFN_io_b_sExp),
    .io_b_sig(mulRawFN_io_b_sig),
    .io_invalidExc(mulRawFN_io_invalidExc),
    .io_rawOut_isNaN(mulRawFN_io_rawOut_isNaN),
    .io_rawOut_isInf(mulRawFN_io_rawOut_isInf),
    .io_rawOut_isZero(mulRawFN_io_rawOut_isZero),
    .io_rawOut_sign(mulRawFN_io_rawOut_sign),
    .io_rawOut_sExp(mulRawFN_io_rawOut_sExp),
    .io_rawOut_sig(mulRawFN_io_rawOut_sig)
  );
  RoundRawFNToRecFN roundRawFNToRecFN ( // @[MulRecFN.scala 121:15]
    .io_invalidExc(roundRawFNToRecFN_io_invalidExc),
    .io_in_isNaN(roundRawFNToRecFN_io_in_isNaN),
    .io_in_isInf(roundRawFNToRecFN_io_in_isInf),
    .io_in_isZero(roundRawFNToRecFN_io_in_isZero),
    .io_in_sign(roundRawFNToRecFN_io_in_sign),
    .io_in_sExp(roundRawFNToRecFN_io_in_sExp),
    .io_in_sig(roundRawFNToRecFN_io_in_sig),
    .io_out(roundRawFNToRecFN_io_out)
  );
  assign io_out = roundRawFNToRecFN_io_out; // @[MulRecFN.scala 127:23]
  assign mulRawFN_io_a_isNaN = _T_4 & io_a[29]; // @[rawFloatFromRecFN.scala 55:33]
  assign mulRawFN_io_a_isInf = _T_4 & ~io_a[29]; // @[rawFloatFromRecFN.scala 56:33]
  assign mulRawFN_io_a_isZero = io_a[31:29] == 3'h0; // @[rawFloatFromRecFN.scala 51:54]
  assign mulRawFN_io_a_sign = io_a[32]; // @[rawFloatFromRecFN.scala 58:25]
  assign mulRawFN_io_a_sExp = {1'b0,$signed(io_a[31:23])}; // @[rawFloatFromRecFN.scala 59:27]
  assign mulRawFN_io_a_sig = {hi,lo}; // @[Cat.scala 30:58]
  assign mulRawFN_io_b_isNaN = _T_17 & io_b[29]; // @[rawFloatFromRecFN.scala 55:33]
  assign mulRawFN_io_b_isInf = _T_17 & ~io_b[29]; // @[rawFloatFromRecFN.scala 56:33]
  assign mulRawFN_io_b_isZero = io_b[31:29] == 3'h0; // @[rawFloatFromRecFN.scala 51:54]
  assign mulRawFN_io_b_sign = io_b[32]; // @[rawFloatFromRecFN.scala 58:25]
  assign mulRawFN_io_b_sExp = {1'b0,$signed(io_b[31:23])}; // @[rawFloatFromRecFN.scala 59:27]
  assign mulRawFN_io_b_sig = {hi_1,lo_1}; // @[Cat.scala 30:58]
  assign roundRawFNToRecFN_io_invalidExc = mulRawFN_io_invalidExc; // @[MulRecFN.scala 122:39]
  assign roundRawFNToRecFN_io_in_isNaN = mulRawFN_io_rawOut_isNaN; // @[MulRecFN.scala 124:39]
  assign roundRawFNToRecFN_io_in_isInf = mulRawFN_io_rawOut_isInf; // @[MulRecFN.scala 124:39]
  assign roundRawFNToRecFN_io_in_isZero = mulRawFN_io_rawOut_isZero; // @[MulRecFN.scala 124:39]
  assign roundRawFNToRecFN_io_in_sign = mulRawFN_io_rawOut_sign; // @[MulRecFN.scala 124:39]
  assign roundRawFNToRecFN_io_in_sExp = mulRawFN_io_rawOut_sExp; // @[MulRecFN.scala 124:39]
  assign roundRawFNToRecFN_io_in_sig = mulRawFN_io_rawOut_sig; // @[MulRecFN.scala 124:39]
endmodule
module MulFBase(
  input         clock,
  input  [31:0] operand0,
  input  [31:0] operand1,
  input         ce,
  output [31:0] result
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
`endif // RANDOMIZE_REG_INIT
  wire [32:0] multiplier_io_a; // @[fpunits.scala 33:28]
  wire [32:0] multiplier_io_b; // @[fpunits.scala 33:28]
  wire [32:0] multiplier_io_out; // @[fpunits.scala 33:28]
  wire  new_clock = clock & ce; // @[fpunits.scala 21:51]
  reg [31:0] operand0_reg; // @[fpunits.scala 25:31]
  reg [31:0] operand1_reg; // @[fpunits.scala 26:31]
  wire  _operand0_rec_T_3 = operand0_reg[30:23] == 8'h0; // @[rawFloatFromFN.scala 50:34]
  wire  _operand0_rec_T_4 = operand0_reg[22:0] == 23'h0; // @[rawFloatFromFN.scala 51:38]
  wire [4:0] _operand0_rec_T_28 = operand0_reg[1] ? 5'h15 : 5'h16; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_29 = operand0_reg[2] ? 5'h14 : _operand0_rec_T_28; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_30 = operand0_reg[3] ? 5'h13 : _operand0_rec_T_29; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_31 = operand0_reg[4] ? 5'h12 : _operand0_rec_T_30; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_32 = operand0_reg[5] ? 5'h11 : _operand0_rec_T_31; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_33 = operand0_reg[6] ? 5'h10 : _operand0_rec_T_32; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_34 = operand0_reg[7] ? 5'hf : _operand0_rec_T_33; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_35 = operand0_reg[8] ? 5'he : _operand0_rec_T_34; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_36 = operand0_reg[9] ? 5'hd : _operand0_rec_T_35; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_37 = operand0_reg[10] ? 5'hc : _operand0_rec_T_36; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_38 = operand0_reg[11] ? 5'hb : _operand0_rec_T_37; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_39 = operand0_reg[12] ? 5'ha : _operand0_rec_T_38; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_40 = operand0_reg[13] ? 5'h9 : _operand0_rec_T_39; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_41 = operand0_reg[14] ? 5'h8 : _operand0_rec_T_40; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_42 = operand0_reg[15] ? 5'h7 : _operand0_rec_T_41; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_43 = operand0_reg[16] ? 5'h6 : _operand0_rec_T_42; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_44 = operand0_reg[17] ? 5'h5 : _operand0_rec_T_43; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_45 = operand0_reg[18] ? 5'h4 : _operand0_rec_T_44; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_46 = operand0_reg[19] ? 5'h3 : _operand0_rec_T_45; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_47 = operand0_reg[20] ? 5'h2 : _operand0_rec_T_46; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_48 = operand0_reg[21] ? 5'h1 : _operand0_rec_T_47; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_49 = operand0_reg[22] ? 5'h0 : _operand0_rec_T_48; // @[Mux.scala 47:69]
  wire [53:0] _GEN_0 = {{31'd0}, operand0_reg[22:0]}; // @[rawFloatFromFN.scala 54:36]
  wire [53:0] _operand0_rec_T_50 = _GEN_0 << _operand0_rec_T_49; // @[rawFloatFromFN.scala 54:36]
  wire [22:0] _operand0_rec_T_52 = {_operand0_rec_T_50[21:0], 1'h0}; // @[rawFloatFromFN.scala 54:64]
  wire [8:0] _GEN_1 = {{4'd0}, _operand0_rec_T_49}; // @[rawFloatFromFN.scala 57:26]
  wire [8:0] _operand0_rec_T_53 = _GEN_1 ^ 9'h1ff; // @[rawFloatFromFN.scala 57:26]
  wire [8:0] _operand0_rec_T_54 = _operand0_rec_T_3 ? _operand0_rec_T_53 : {{1'd0}, operand0_reg[30:23]}; // @[rawFloatFromFN.scala 56:16]
  wire [1:0] _operand0_rec_T_55 = _operand0_rec_T_3 ? 2'h2 : 2'h1; // @[rawFloatFromFN.scala 60:27]
  wire [7:0] _GEN_2 = {{6'd0}, _operand0_rec_T_55}; // @[rawFloatFromFN.scala 60:22]
  wire [7:0] _operand0_rec_T_56 = 8'h80 | _GEN_2; // @[rawFloatFromFN.scala 60:22]
  wire [8:0] _GEN_3 = {{1'd0}, _operand0_rec_T_56}; // @[rawFloatFromFN.scala 59:15]
  wire [8:0] _operand0_rec_T_58 = _operand0_rec_T_54 + _GEN_3; // @[rawFloatFromFN.scala 59:15]
  wire  _operand0_rec_T_59 = _operand0_rec_T_3 & _operand0_rec_T_4; // @[rawFloatFromFN.scala 62:34]
  wire  _operand0_rec_T_61 = _operand0_rec_T_58[8:7] == 2'h3; // @[rawFloatFromFN.scala 63:62]
  wire  _operand0_rec_T_63 = _operand0_rec_T_61 & ~_operand0_rec_T_4; // @[rawFloatFromFN.scala 66:33]
  wire [9:0] _operand0_rec_T_66 = {1'b0,$signed(_operand0_rec_T_58)}; // @[rawFloatFromFN.scala 70:48]
  wire  operand0_rec_hi_lo = ~_operand0_rec_T_59; // @[rawFloatFromFN.scala 72:29]
  wire [22:0] operand0_rec_lo = _operand0_rec_T_3 ? _operand0_rec_T_52 : operand0_reg[22:0]; // @[rawFloatFromFN.scala 72:42]
  wire [24:0] _operand0_rec_T_67 = {1'h0,operand0_rec_hi_lo,operand0_rec_lo}; // @[Cat.scala 30:58]
  wire [2:0] _operand0_rec_T_69 = _operand0_rec_T_59 ? 3'h0 : _operand0_rec_T_66[8:6]; // @[recFNFromFN.scala 48:16]
  wire [2:0] _GEN_4 = {{2'd0}, _operand0_rec_T_63}; // @[recFNFromFN.scala 48:79]
  wire [2:0] operand0_rec_hi_lo_1 = _operand0_rec_T_69 | _GEN_4; // @[recFNFromFN.scala 48:79]
  wire [5:0] operand0_rec_lo_hi = _operand0_rec_T_66[5:0]; // @[recFNFromFN.scala 50:23]
  wire [22:0] operand0_rec_lo_lo = _operand0_rec_T_67[22:0]; // @[recFNFromFN.scala 51:22]
  wire [28:0] operand0_rec_lo_1 = {operand0_rec_lo_hi,operand0_rec_lo_lo}; // @[Cat.scala 30:58]
  wire [3:0] operand0_rec_hi_1 = {operand0_reg[31],operand0_rec_hi_lo_1}; // @[Cat.scala 30:58]
  wire  _operand1_rec_T_3 = operand1_reg[30:23] == 8'h0; // @[rawFloatFromFN.scala 50:34]
  wire  _operand1_rec_T_4 = operand1_reg[22:0] == 23'h0; // @[rawFloatFromFN.scala 51:38]
  wire [4:0] _operand1_rec_T_28 = operand1_reg[1] ? 5'h15 : 5'h16; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_29 = operand1_reg[2] ? 5'h14 : _operand1_rec_T_28; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_30 = operand1_reg[3] ? 5'h13 : _operand1_rec_T_29; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_31 = operand1_reg[4] ? 5'h12 : _operand1_rec_T_30; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_32 = operand1_reg[5] ? 5'h11 : _operand1_rec_T_31; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_33 = operand1_reg[6] ? 5'h10 : _operand1_rec_T_32; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_34 = operand1_reg[7] ? 5'hf : _operand1_rec_T_33; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_35 = operand1_reg[8] ? 5'he : _operand1_rec_T_34; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_36 = operand1_reg[9] ? 5'hd : _operand1_rec_T_35; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_37 = operand1_reg[10] ? 5'hc : _operand1_rec_T_36; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_38 = operand1_reg[11] ? 5'hb : _operand1_rec_T_37; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_39 = operand1_reg[12] ? 5'ha : _operand1_rec_T_38; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_40 = operand1_reg[13] ? 5'h9 : _operand1_rec_T_39; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_41 = operand1_reg[14] ? 5'h8 : _operand1_rec_T_40; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_42 = operand1_reg[15] ? 5'h7 : _operand1_rec_T_41; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_43 = operand1_reg[16] ? 5'h6 : _operand1_rec_T_42; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_44 = operand1_reg[17] ? 5'h5 : _operand1_rec_T_43; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_45 = operand1_reg[18] ? 5'h4 : _operand1_rec_T_44; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_46 = operand1_reg[19] ? 5'h3 : _operand1_rec_T_45; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_47 = operand1_reg[20] ? 5'h2 : _operand1_rec_T_46; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_48 = operand1_reg[21] ? 5'h1 : _operand1_rec_T_47; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_49 = operand1_reg[22] ? 5'h0 : _operand1_rec_T_48; // @[Mux.scala 47:69]
  wire [53:0] _GEN_5 = {{31'd0}, operand1_reg[22:0]}; // @[rawFloatFromFN.scala 54:36]
  wire [53:0] _operand1_rec_T_50 = _GEN_5 << _operand1_rec_T_49; // @[rawFloatFromFN.scala 54:36]
  wire [22:0] _operand1_rec_T_52 = {_operand1_rec_T_50[21:0], 1'h0}; // @[rawFloatFromFN.scala 54:64]
  wire [8:0] _GEN_6 = {{4'd0}, _operand1_rec_T_49}; // @[rawFloatFromFN.scala 57:26]
  wire [8:0] _operand1_rec_T_53 = _GEN_6 ^ 9'h1ff; // @[rawFloatFromFN.scala 57:26]
  wire [8:0] _operand1_rec_T_54 = _operand1_rec_T_3 ? _operand1_rec_T_53 : {{1'd0}, operand1_reg[30:23]}; // @[rawFloatFromFN.scala 56:16]
  wire [1:0] _operand1_rec_T_55 = _operand1_rec_T_3 ? 2'h2 : 2'h1; // @[rawFloatFromFN.scala 60:27]
  wire [7:0] _GEN_7 = {{6'd0}, _operand1_rec_T_55}; // @[rawFloatFromFN.scala 60:22]
  wire [7:0] _operand1_rec_T_56 = 8'h80 | _GEN_7; // @[rawFloatFromFN.scala 60:22]
  wire [8:0] _GEN_8 = {{1'd0}, _operand1_rec_T_56}; // @[rawFloatFromFN.scala 59:15]
  wire [8:0] _operand1_rec_T_58 = _operand1_rec_T_54 + _GEN_8; // @[rawFloatFromFN.scala 59:15]
  wire  _operand1_rec_T_59 = _operand1_rec_T_3 & _operand1_rec_T_4; // @[rawFloatFromFN.scala 62:34]
  wire  _operand1_rec_T_61 = _operand1_rec_T_58[8:7] == 2'h3; // @[rawFloatFromFN.scala 63:62]
  wire  _operand1_rec_T_63 = _operand1_rec_T_61 & ~_operand1_rec_T_4; // @[rawFloatFromFN.scala 66:33]
  wire [9:0] _operand1_rec_T_66 = {1'b0,$signed(_operand1_rec_T_58)}; // @[rawFloatFromFN.scala 70:48]
  wire  operand1_rec_hi_lo = ~_operand1_rec_T_59; // @[rawFloatFromFN.scala 72:29]
  wire [22:0] operand1_rec_lo = _operand1_rec_T_3 ? _operand1_rec_T_52 : operand1_reg[22:0]; // @[rawFloatFromFN.scala 72:42]
  wire [24:0] _operand1_rec_T_67 = {1'h0,operand1_rec_hi_lo,operand1_rec_lo}; // @[Cat.scala 30:58]
  wire [2:0] _operand1_rec_T_69 = _operand1_rec_T_59 ? 3'h0 : _operand1_rec_T_66[8:6]; // @[recFNFromFN.scala 48:16]
  wire [2:0] _GEN_9 = {{2'd0}, _operand1_rec_T_63}; // @[recFNFromFN.scala 48:79]
  wire [2:0] operand1_rec_hi_lo_1 = _operand1_rec_T_69 | _GEN_9; // @[recFNFromFN.scala 48:79]
  wire [5:0] operand1_rec_lo_hi = _operand1_rec_T_66[5:0]; // @[recFNFromFN.scala 50:23]
  wire [22:0] operand1_rec_lo_lo = _operand1_rec_T_67[22:0]; // @[recFNFromFN.scala 51:22]
  wire [28:0] operand1_rec_lo_1 = {operand1_rec_lo_hi,operand1_rec_lo_lo}; // @[Cat.scala 30:58]
  wire [3:0] operand1_rec_hi_1 = {operand1_reg[31],operand1_rec_hi_lo_1}; // @[Cat.scala 30:58]
  wire  _output_T_2 = multiplier_io_out[31:29] == 3'h0; // @[rawFloatFromRecFN.scala 51:54]
  wire  _output_T_4 = multiplier_io_out[31:30] == 2'h3; // @[rawFloatFromRecFN.scala 52:54]
  wire  _output_T_6 = _output_T_4 & multiplier_io_out[29]; // @[rawFloatFromRecFN.scala 55:33]
  wire  _output_T_9 = _output_T_4 & ~multiplier_io_out[29]; // @[rawFloatFromRecFN.scala 56:33]
  wire [9:0] _output_T_11 = {1'b0,$signed(multiplier_io_out[31:23])}; // @[rawFloatFromRecFN.scala 59:27]
  wire  output_hi_lo = ~_output_T_2; // @[rawFloatFromRecFN.scala 60:39]
  wire [22:0] output_lo = multiplier_io_out[22:0]; // @[rawFloatFromRecFN.scala 60:51]
  wire [24:0] _output_T_12 = {1'h0,output_hi_lo,output_lo}; // @[Cat.scala 30:58]
  wire  _output_T_13 = $signed(_output_T_11) < 10'sh82; // @[fNFromRecFN.scala 50:39]
  wire [4:0] _output_T_16 = 5'h1 - _output_T_11[4:0]; // @[fNFromRecFN.scala 51:39]
  wire [23:0] _output_T_18 = _output_T_12[24:1] >> _output_T_16; // @[fNFromRecFN.scala 52:42]
  wire [7:0] _output_T_22 = _output_T_11[7:0] - 8'h81; // @[fNFromRecFN.scala 57:45]
  wire [7:0] _output_T_23 = _output_T_13 ? 8'h0 : _output_T_22; // @[fNFromRecFN.scala 55:16]
  wire  _output_T_24 = _output_T_6 | _output_T_9; // @[fNFromRecFN.scala 59:44]
  wire [7:0] _output_T_26 = _output_T_24 ? 8'hff : 8'h0; // @[Bitwise.scala 72:12]
  wire [7:0] output_hi_lo_1 = _output_T_23 | _output_T_26; // @[fNFromRecFN.scala 59:15]
  wire [22:0] _output_T_28 = _output_T_9 ? 23'h0 : _output_T_12[22:0]; // @[fNFromRecFN.scala 63:20]
  wire [22:0] output_lo_1 = _output_T_13 ? _output_T_18[22:0] : _output_T_28; // @[fNFromRecFN.scala 61:16]
  wire [8:0] output_hi_1 = {multiplier_io_out[32],output_hi_lo_1}; // @[Cat.scala 30:58]
  reg [31:0] shiftRegs_0; // @[fpunits.scala 46:26]
  reg [31:0] shiftRegs_1; // @[fpunits.scala 46:26]
  MulRecFN multiplier ( // @[fpunits.scala 33:28]
    .io_a(multiplier_io_a),
    .io_b(multiplier_io_b),
    .io_out(multiplier_io_out)
  );
  assign result = shiftRegs_1; // @[fpunits.scala 52:14]
  assign multiplier_io_a = {operand0_rec_hi_1,operand0_rec_lo_1}; // @[Cat.scala 30:58]
  assign multiplier_io_b = {operand1_rec_hi_1,operand1_rec_lo_1}; // @[Cat.scala 30:58]
  always @(posedge new_clock) begin
    operand0_reg <= operand0; // @[fpunits.scala 25:31]
    operand1_reg <= operand1; // @[fpunits.scala 26:31]
    shiftRegs_0 <= {output_hi_1,output_lo_1}; // @[Cat.scala 30:58]
    shiftRegs_1 <= shiftRegs_0; // @[fpunits.scala 49:26]
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  operand0_reg = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  operand1_reg = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  shiftRegs_0 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  shiftRegs_1 = _RAND_3[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module AddRawFN(
  input         io_a_isNaN,
  input         io_a_isInf,
  input         io_a_isZero,
  input         io_a_sign,
  input  [9:0]  io_a_sExp,
  input  [24:0] io_a_sig,
  input         io_b_isNaN,
  input         io_b_isInf,
  input         io_b_isZero,
  input         io_b_sign,
  input  [9:0]  io_b_sExp,
  input  [24:0] io_b_sig,
  output        io_invalidExc,
  output        io_rawOut_isNaN,
  output        io_rawOut_isInf,
  output        io_rawOut_isZero,
  output        io_rawOut_sign,
  output [9:0]  io_rawOut_sExp,
  output [26:0] io_rawOut_sig
);
  wire  eqSigns = io_a_sign == io_b_sign; // @[AddRecFN.scala 61:29]
  wire [9:0] sDiffExps = $signed(io_a_sExp) - $signed(io_b_sExp); // @[AddRecFN.scala 63:31]
  wire  _T_2 = $signed(sDiffExps) < 10'sh0; // @[AddRecFN.scala 64:41]
  wire [9:0] _T_5 = $signed(io_b_sExp) - $signed(io_a_sExp); // @[AddRecFN.scala 64:58]
  wire [9:0] _T_6 = $signed(sDiffExps) < 10'sh0 ? $signed(_T_5) : $signed(sDiffExps); // @[AddRecFN.scala 64:30]
  wire [4:0] modNatAlignDist = _T_6[4:0]; // @[AddRecFN.scala 64:81]
  wire [4:0] _T_7 = sDiffExps[9:5]; // @[AddRecFN.scala 66:19]
  wire  _T_13 = $signed(_T_7) != -5'sh1 | sDiffExps[4:0] == 5'h0; // @[AddRecFN.scala 67:51]
  wire  isMaxAlign = $signed(_T_7) != 5'sh0 & _T_13; // @[AddRecFN.scala 66:45]
  wire [4:0] alignDist = isMaxAlign ? 5'h1f : modNatAlignDist; // @[AddRecFN.scala 68:24]
  wire  _T_14 = ~eqSigns; // @[AddRecFN.scala 69:24]
  wire  closeSubMags = ~eqSigns & ~isMaxAlign & modNatAlignDist <= 5'h1; // @[AddRecFN.scala 69:48]
  wire  _T_18 = 10'sh0 <= $signed(sDiffExps); // @[AddRecFN.scala 73:18]
  wire [26:0] _T_21 = {io_a_sig, 2'h0}; // @[AddRecFN.scala 73:58]
  wire [26:0] _T_22 = 10'sh0 <= $signed(sDiffExps) & sDiffExps[0] ? _T_21 : 27'h0; // @[AddRecFN.scala 73:12]
  wire [25:0] _T_27 = {io_a_sig, 1'h0}; // @[AddRecFN.scala 74:58]
  wire [25:0] _T_28 = _T_18 & ~sDiffExps[0] ? _T_27 : 26'h0; // @[AddRecFN.scala 74:12]
  wire [26:0] _GEN_0 = {{1'd0}, _T_28}; // @[AddRecFN.scala 73:68]
  wire [26:0] _T_29 = _T_22 | _GEN_0; // @[AddRecFN.scala 73:68]
  wire [24:0] _T_31 = _T_2 ? io_a_sig : 25'h0; // @[AddRecFN.scala 75:12]
  wire [26:0] _GEN_1 = {{2'd0}, _T_31}; // @[AddRecFN.scala 74:68]
  wire [26:0] _T_32 = _T_29 | _GEN_1; // @[AddRecFN.scala 76:43]
  wire [25:0] _T_34 = {io_b_sig, 1'h0}; // @[AddRecFN.scala 76:66]
  wire [26:0] _GEN_2 = {{1{_T_34[25]}},_T_34}; // @[AddRecFN.scala 76:50]
  wire [26:0] close_sSigSum = $signed(_T_32) - $signed(_GEN_2); // @[AddRecFN.scala 76:50]
  wire  _T_37 = $signed(close_sSigSum) < 27'sh0; // @[AddRecFN.scala 77:42]
  wire [26:0] _T_40 = 27'sh0 - $signed(close_sSigSum); // @[AddRecFN.scala 77:49]
  wire [26:0] _T_41 = $signed(close_sSigSum) < 27'sh0 ? $signed(_T_40) : $signed(close_sSigSum); // @[AddRecFN.scala 77:27]
  wire [25:0] close_sigSum = _T_41[25:0]; // @[AddRecFN.scala 77:79]
  wire  _T_43 = |close_sigSum[1:0]; // @[primitives.scala 104:54]
  wire  _T_45 = |close_sigSum[3:2]; // @[primitives.scala 104:54]
  wire  _T_47 = |close_sigSum[5:4]; // @[primitives.scala 104:54]
  wire  _T_49 = |close_sigSum[7:6]; // @[primitives.scala 104:54]
  wire  _T_51 = |close_sigSum[9:8]; // @[primitives.scala 104:54]
  wire  _T_53 = |close_sigSum[11:10]; // @[primitives.scala 104:54]
  wire  _T_55 = |close_sigSum[13:12]; // @[primitives.scala 104:54]
  wire  _T_57 = |close_sigSum[15:14]; // @[primitives.scala 104:54]
  wire  _T_59 = |close_sigSum[17:16]; // @[primitives.scala 104:54]
  wire  _T_61 = |close_sigSum[19:18]; // @[primitives.scala 104:54]
  wire  _T_63 = |close_sigSum[21:20]; // @[primitives.scala 104:54]
  wire  _T_65 = |close_sigSum[23:22]; // @[primitives.scala 104:54]
  wire  _T_67 = |close_sigSum[25:24]; // @[primitives.scala 107:57]
  wire [5:0] lo = {_T_53,_T_51,_T_49,_T_47,_T_45,_T_43}; // @[primitives.scala 108:20]
  wire [12:0] close_reduced2SigSum = {_T_67,_T_65,_T_63,_T_61,_T_59,_T_57,_T_55,lo}; // @[primitives.scala 108:20]
  wire [3:0] _T_81 = close_reduced2SigSum[1] ? 4'hb : 4'hc; // @[Mux.scala 47:69]
  wire [3:0] _T_82 = close_reduced2SigSum[2] ? 4'ha : _T_81; // @[Mux.scala 47:69]
  wire [3:0] _T_83 = close_reduced2SigSum[3] ? 4'h9 : _T_82; // @[Mux.scala 47:69]
  wire [3:0] _T_84 = close_reduced2SigSum[4] ? 4'h8 : _T_83; // @[Mux.scala 47:69]
  wire [3:0] _T_85 = close_reduced2SigSum[5] ? 4'h7 : _T_84; // @[Mux.scala 47:69]
  wire [3:0] _T_86 = close_reduced2SigSum[6] ? 4'h6 : _T_85; // @[Mux.scala 47:69]
  wire [3:0] _T_87 = close_reduced2SigSum[7] ? 4'h5 : _T_86; // @[Mux.scala 47:69]
  wire [3:0] _T_88 = close_reduced2SigSum[8] ? 4'h4 : _T_87; // @[Mux.scala 47:69]
  wire [3:0] _T_89 = close_reduced2SigSum[9] ? 4'h3 : _T_88; // @[Mux.scala 47:69]
  wire [3:0] _T_90 = close_reduced2SigSum[10] ? 4'h2 : _T_89; // @[Mux.scala 47:69]
  wire [3:0] _T_91 = close_reduced2SigSum[11] ? 4'h1 : _T_90; // @[Mux.scala 47:69]
  wire [3:0] close_normDistReduced2 = close_reduced2SigSum[12] ? 4'h0 : _T_91; // @[Mux.scala 47:69]
  wire [4:0] close_nearNormDist = {close_normDistReduced2, 1'h0}; // @[AddRecFN.scala 81:53]
  wire [56:0] _GEN_3 = {{31'd0}, close_sigSum}; // @[AddRecFN.scala 82:38]
  wire [56:0] _T_93 = _GEN_3 << close_nearNormDist; // @[AddRecFN.scala 82:38]
  wire [57:0] _T_94 = {_T_93, 1'h0}; // @[AddRecFN.scala 82:59]
  wire [26:0] close_sigOut = _T_94[26:0]; // @[AddRecFN.scala 82:63]
  wire  close_totalCancellation = ~(|close_sigOut[26:25]); // @[AddRecFN.scala 83:35]
  wire  close_notTotalCancellation_signOut = io_a_sign ^ _T_37; // @[AddRecFN.scala 84:56]
  wire  far_signOut = _T_2 ? io_b_sign : io_a_sign; // @[AddRecFN.scala 87:26]
  wire [24:0] _T_100 = _T_2 ? io_b_sig : io_a_sig; // @[AddRecFN.scala 88:29]
  wire [23:0] far_sigLarger = _T_100[23:0]; // @[AddRecFN.scala 88:66]
  wire [24:0] _T_102 = _T_2 ? io_a_sig : io_b_sig; // @[AddRecFN.scala 89:29]
  wire [23:0] far_sigSmaller = _T_102[23:0]; // @[AddRecFN.scala 89:66]
  wire [28:0] _T_103 = {far_sigSmaller, 5'h0}; // @[AddRecFN.scala 90:52]
  wire [28:0] far_mainAlignedSigSmaller = _T_103 >> alignDist; // @[AddRecFN.scala 90:56]
  wire [25:0] _T_104 = {far_sigSmaller, 2'h0}; // @[AddRecFN.scala 91:60]
  wire  _T_106 = |_T_104[3:0]; // @[primitives.scala 121:54]
  wire  _T_108 = |_T_104[7:4]; // @[primitives.scala 121:54]
  wire  _T_110 = |_T_104[11:8]; // @[primitives.scala 121:54]
  wire  _T_112 = |_T_104[15:12]; // @[primitives.scala 121:54]
  wire  _T_114 = |_T_104[19:16]; // @[primitives.scala 121:54]
  wire  _T_116 = |_T_104[23:20]; // @[primitives.scala 121:54]
  wire  _T_118 = |_T_104[25:24]; // @[primitives.scala 124:57]
  wire [6:0] far_reduced4SigSmaller = {_T_118,_T_116,_T_114,_T_112,_T_110,_T_108,_T_106}; // @[primitives.scala 125:20]
  wire [8:0] _T_120 = 9'sh100 >>> alignDist[4:2]; // @[primitives.scala 77:58]
  wire  hi_2 = _T_120[1]; // @[Bitwise.scala 109:18]
  wire  lo_2 = _T_120[2]; // @[Bitwise.scala 109:44]
  wire  hi_4 = _T_120[3]; // @[Bitwise.scala 109:18]
  wire  lo_3 = _T_120[4]; // @[Bitwise.scala 109:44]
  wire  hi_6 = _T_120[5]; // @[Bitwise.scala 109:18]
  wire  lo_5 = _T_120[6]; // @[Bitwise.scala 109:44]
  wire  lo_6 = _T_120[7]; // @[Bitwise.scala 109:44]
  wire [6:0] far_roundExtraMask = {hi_2,lo_2,hi_4,lo_3,hi_6,lo_5,lo_6}; // @[Cat.scala 30:58]
  wire [25:0] hi_8 = far_mainAlignedSigSmaller[28:3]; // @[AddRecFN.scala 94:38]
  wire [6:0] _T_129 = far_reduced4SigSmaller & far_roundExtraMask; // @[AddRecFN.scala 95:76]
  wire  lo_8 = |far_mainAlignedSigSmaller[2:0] | |_T_129; // @[AddRecFN.scala 95:49]
  wire [26:0] far_alignedSigSmaller = {hi_8,lo_8}; // @[Cat.scala 30:58]
  wire [26:0] lo_9 = ~far_alignedSigSmaller; // @[AddRecFN.scala 97:62]
  wire [27:0] _T_131 = {1'h1,lo_9}; // @[Cat.scala 30:58]
  wire [27:0] far_negAlignedSigSmaller = _T_14 ? _T_131 : {{1'd0}, far_alignedSigSmaller}; // @[AddRecFN.scala 97:39]
  wire [26:0] _T_132 = {far_sigLarger, 3'h0}; // @[AddRecFN.scala 98:36]
  wire [27:0] _GEN_4 = {{1'd0}, _T_132}; // @[AddRecFN.scala 98:41]
  wire [27:0] _T_134 = _GEN_4 + far_negAlignedSigSmaller; // @[AddRecFN.scala 98:41]
  wire [27:0] _GEN_5 = {{27'd0}, _T_14}; // @[AddRecFN.scala 98:68]
  wire [27:0] far_sigSum = _T_134 + _GEN_5; // @[AddRecFN.scala 98:68]
  wire [26:0] _GEN_6 = {{26'd0}, far_sigSum[0]}; // @[AddRecFN.scala 99:67]
  wire [26:0] _T_138 = far_sigSum[27:1] | _GEN_6; // @[AddRecFN.scala 99:67]
  wire [27:0] _T_139 = _T_14 ? far_sigSum : {{1'd0}, _T_138}; // @[AddRecFN.scala 99:25]
  wire [26:0] far_sigOut = _T_139[26:0]; // @[AddRecFN.scala 99:83]
  wire  notSigNaN_invalidExc = io_a_isInf & io_b_isInf & _T_14; // @[AddRecFN.scala 102:57]
  wire  notNaN_isInfOut = io_a_isInf | io_b_isInf; // @[AddRecFN.scala 103:38]
  wire  addZeros = io_a_isZero & io_b_isZero; // @[AddRecFN.scala 104:32]
  wire  notNaN_specialCase = notNaN_isInfOut | addZeros; // @[AddRecFN.scala 105:46]
  wire  _T_146 = io_a_isInf & io_a_sign; // @[AddRecFN.scala 109:39]
  wire  _T_147 = eqSigns & io_a_sign | _T_146; // @[AddRecFN.scala 108:63]
  wire  _T_148 = io_b_isInf & io_b_sign; // @[AddRecFN.scala 110:39]
  wire  _T_149 = _T_147 | _T_148; // @[AddRecFN.scala 109:63]
  wire  _T_154 = ~notNaN_specialCase; // @[AddRecFN.scala 112:10]
  wire  _T_157 = ~notNaN_specialCase & closeSubMags & ~close_totalCancellation; // @[AddRecFN.scala 112:46]
  wire  _T_158 = _T_157 & close_notTotalCancellation_signOut; // @[AddRecFN.scala 113:38]
  wire  _T_159 = _T_149 | _T_158; // @[AddRecFN.scala 111:63]
  wire  _T_163 = _T_154 & ~closeSubMags & far_signOut; // @[AddRecFN.scala 114:47]
  wire [9:0] _T_166 = closeSubMags | _T_2 ? $signed(io_b_sExp) : $signed(io_a_sExp); // @[AddRecFN.scala 116:13]
  wire [4:0] _T_167 = closeSubMags ? close_nearNormDist : {{4'd0}, _T_14}; // @[AddRecFN.scala 117:18]
  wire [5:0] _T_168 = {1'b0,$signed(_T_167)}; // @[AddRecFN.scala 117:66]
  wire [9:0] _GEN_7 = {{4{_T_168[5]}},_T_168}; // @[AddRecFN.scala 117:13]
  wire  _T_173 = io_a_isNaN & ~io_a_sig[22]; // @[common.scala 84:46]
  wire  _T_176 = io_b_isNaN & ~io_b_sig[22]; // @[common.scala 84:46]
  assign io_invalidExc = _T_173 | _T_176 | notSigNaN_invalidExc; // @[AddRecFN.scala 121:71]
  assign io_rawOut_isNaN = io_a_isNaN | io_b_isNaN; // @[AddRecFN.scala 125:35]
  assign io_rawOut_isInf = io_a_isInf | io_b_isInf; // @[AddRecFN.scala 103:38]
  assign io_rawOut_isZero = addZeros | ~notNaN_isInfOut & closeSubMags & close_totalCancellation; // @[AddRecFN.scala 106:37]
  assign io_rawOut_sign = _T_159 | _T_163; // @[AddRecFN.scala 113:77]
  assign io_rawOut_sExp = $signed(_T_166) - $signed(_GEN_7); // @[AddRecFN.scala 117:13]
  assign io_rawOut_sig = closeSubMags ? close_sigOut : far_sigOut; // @[AddRecFN.scala 118:28]
endmodule
module AddRecFN(
  input  [32:0] io_a,
  input  [32:0] io_b,
  output [32:0] io_out
);
  wire  addRawFN_io_a_isNaN; // @[AddRecFN.scala 147:26]
  wire  addRawFN_io_a_isInf; // @[AddRecFN.scala 147:26]
  wire  addRawFN_io_a_isZero; // @[AddRecFN.scala 147:26]
  wire  addRawFN_io_a_sign; // @[AddRecFN.scala 147:26]
  wire [9:0] addRawFN_io_a_sExp; // @[AddRecFN.scala 147:26]
  wire [24:0] addRawFN_io_a_sig; // @[AddRecFN.scala 147:26]
  wire  addRawFN_io_b_isNaN; // @[AddRecFN.scala 147:26]
  wire  addRawFN_io_b_isInf; // @[AddRecFN.scala 147:26]
  wire  addRawFN_io_b_isZero; // @[AddRecFN.scala 147:26]
  wire  addRawFN_io_b_sign; // @[AddRecFN.scala 147:26]
  wire [9:0] addRawFN_io_b_sExp; // @[AddRecFN.scala 147:26]
  wire [24:0] addRawFN_io_b_sig; // @[AddRecFN.scala 147:26]
  wire  addRawFN_io_invalidExc; // @[AddRecFN.scala 147:26]
  wire  addRawFN_io_rawOut_isNaN; // @[AddRecFN.scala 147:26]
  wire  addRawFN_io_rawOut_isInf; // @[AddRecFN.scala 147:26]
  wire  addRawFN_io_rawOut_isZero; // @[AddRecFN.scala 147:26]
  wire  addRawFN_io_rawOut_sign; // @[AddRecFN.scala 147:26]
  wire [9:0] addRawFN_io_rawOut_sExp; // @[AddRecFN.scala 147:26]
  wire [26:0] addRawFN_io_rawOut_sig; // @[AddRecFN.scala 147:26]
  wire  roundRawFNToRecFN_io_invalidExc; // @[AddRecFN.scala 157:15]
  wire  roundRawFNToRecFN_io_in_isNaN; // @[AddRecFN.scala 157:15]
  wire  roundRawFNToRecFN_io_in_isInf; // @[AddRecFN.scala 157:15]
  wire  roundRawFNToRecFN_io_in_isZero; // @[AddRecFN.scala 157:15]
  wire  roundRawFNToRecFN_io_in_sign; // @[AddRecFN.scala 157:15]
  wire [9:0] roundRawFNToRecFN_io_in_sExp; // @[AddRecFN.scala 157:15]
  wire [26:0] roundRawFNToRecFN_io_in_sig; // @[AddRecFN.scala 157:15]
  wire [32:0] roundRawFNToRecFN_io_out; // @[AddRecFN.scala 157:15]
  wire  _T_2 = io_a[31:29] == 3'h0; // @[rawFloatFromRecFN.scala 51:54]
  wire  _T_4 = io_a[31:30] == 2'h3; // @[rawFloatFromRecFN.scala 52:54]
  wire  hi_lo = ~_T_2; // @[rawFloatFromRecFN.scala 60:39]
  wire [22:0] lo = io_a[22:0]; // @[rawFloatFromRecFN.scala 60:51]
  wire [1:0] hi = {1'h0,hi_lo}; // @[Cat.scala 30:58]
  wire  _T_15 = io_b[31:29] == 3'h0; // @[rawFloatFromRecFN.scala 51:54]
  wire  _T_17 = io_b[31:30] == 2'h3; // @[rawFloatFromRecFN.scala 52:54]
  wire  hi_lo_1 = ~_T_15; // @[rawFloatFromRecFN.scala 60:39]
  wire [22:0] lo_1 = io_b[22:0]; // @[rawFloatFromRecFN.scala 60:51]
  wire [1:0] hi_1 = {1'h0,hi_lo_1}; // @[Cat.scala 30:58]
  AddRawFN addRawFN ( // @[AddRecFN.scala 147:26]
    .io_a_isNaN(addRawFN_io_a_isNaN),
    .io_a_isInf(addRawFN_io_a_isInf),
    .io_a_isZero(addRawFN_io_a_isZero),
    .io_a_sign(addRawFN_io_a_sign),
    .io_a_sExp(addRawFN_io_a_sExp),
    .io_a_sig(addRawFN_io_a_sig),
    .io_b_isNaN(addRawFN_io_b_isNaN),
    .io_b_isInf(addRawFN_io_b_isInf),
    .io_b_isZero(addRawFN_io_b_isZero),
    .io_b_sign(addRawFN_io_b_sign),
    .io_b_sExp(addRawFN_io_b_sExp),
    .io_b_sig(addRawFN_io_b_sig),
    .io_invalidExc(addRawFN_io_invalidExc),
    .io_rawOut_isNaN(addRawFN_io_rawOut_isNaN),
    .io_rawOut_isInf(addRawFN_io_rawOut_isInf),
    .io_rawOut_isZero(addRawFN_io_rawOut_isZero),
    .io_rawOut_sign(addRawFN_io_rawOut_sign),
    .io_rawOut_sExp(addRawFN_io_rawOut_sExp),
    .io_rawOut_sig(addRawFN_io_rawOut_sig)
  );
  RoundRawFNToRecFN roundRawFNToRecFN ( // @[AddRecFN.scala 157:15]
    .io_invalidExc(roundRawFNToRecFN_io_invalidExc),
    .io_in_isNaN(roundRawFNToRecFN_io_in_isNaN),
    .io_in_isInf(roundRawFNToRecFN_io_in_isInf),
    .io_in_isZero(roundRawFNToRecFN_io_in_isZero),
    .io_in_sign(roundRawFNToRecFN_io_in_sign),
    .io_in_sExp(roundRawFNToRecFN_io_in_sExp),
    .io_in_sig(roundRawFNToRecFN_io_in_sig),
    .io_out(roundRawFNToRecFN_io_out)
  );
  assign io_out = roundRawFNToRecFN_io_out; // @[AddRecFN.scala 163:23]
  assign addRawFN_io_a_isNaN = _T_4 & io_a[29]; // @[rawFloatFromRecFN.scala 55:33]
  assign addRawFN_io_a_isInf = _T_4 & ~io_a[29]; // @[rawFloatFromRecFN.scala 56:33]
  assign addRawFN_io_a_isZero = io_a[31:29] == 3'h0; // @[rawFloatFromRecFN.scala 51:54]
  assign addRawFN_io_a_sign = io_a[32]; // @[rawFloatFromRecFN.scala 58:25]
  assign addRawFN_io_a_sExp = {1'b0,$signed(io_a[31:23])}; // @[rawFloatFromRecFN.scala 59:27]
  assign addRawFN_io_a_sig = {hi,lo}; // @[Cat.scala 30:58]
  assign addRawFN_io_b_isNaN = _T_17 & io_b[29]; // @[rawFloatFromRecFN.scala 55:33]
  assign addRawFN_io_b_isInf = _T_17 & ~io_b[29]; // @[rawFloatFromRecFN.scala 56:33]
  assign addRawFN_io_b_isZero = io_b[31:29] == 3'h0; // @[rawFloatFromRecFN.scala 51:54]
  assign addRawFN_io_b_sign = io_b[32]; // @[rawFloatFromRecFN.scala 58:25]
  assign addRawFN_io_b_sExp = {1'b0,$signed(io_b[31:23])}; // @[rawFloatFromRecFN.scala 59:27]
  assign addRawFN_io_b_sig = {hi_1,lo_1}; // @[Cat.scala 30:58]
  assign roundRawFNToRecFN_io_invalidExc = addRawFN_io_invalidExc; // @[AddRecFN.scala 158:39]
  assign roundRawFNToRecFN_io_in_isNaN = addRawFN_io_rawOut_isNaN; // @[AddRecFN.scala 160:39]
  assign roundRawFNToRecFN_io_in_isInf = addRawFN_io_rawOut_isInf; // @[AddRecFN.scala 160:39]
  assign roundRawFNToRecFN_io_in_isZero = addRawFN_io_rawOut_isZero; // @[AddRecFN.scala 160:39]
  assign roundRawFNToRecFN_io_in_sign = addRawFN_io_rawOut_sign; // @[AddRecFN.scala 160:39]
  assign roundRawFNToRecFN_io_in_sExp = addRawFN_io_rawOut_sExp; // @[AddRecFN.scala 160:39]
  assign roundRawFNToRecFN_io_in_sig = addRawFN_io_rawOut_sig; // @[AddRecFN.scala 160:39]
endmodule
module AddSubFBase(
  input         clock,
  input  [31:0] operand0,
  input  [31:0] operand1,
  input         ce,
  output [31:0] result
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
`endif // RANDOMIZE_REG_INIT
  wire [32:0] adder_io_a; // @[fpunits.scala 75:23]
  wire [32:0] adder_io_b; // @[fpunits.scala 75:23]
  wire [32:0] adder_io_out; // @[fpunits.scala 75:23]
  wire  new_clock = clock & ce; // @[fpunits.scala 65:51]
  reg [31:0] operand0_reg; // @[fpunits.scala 67:31]
  reg [31:0] operand1_reg; // @[fpunits.scala 68:31]
  wire  _operand0_rec_T_3 = operand0_reg[30:23] == 8'h0; // @[rawFloatFromFN.scala 50:34]
  wire  _operand0_rec_T_4 = operand0_reg[22:0] == 23'h0; // @[rawFloatFromFN.scala 51:38]
  wire [4:0] _operand0_rec_T_28 = operand0_reg[1] ? 5'h15 : 5'h16; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_29 = operand0_reg[2] ? 5'h14 : _operand0_rec_T_28; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_30 = operand0_reg[3] ? 5'h13 : _operand0_rec_T_29; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_31 = operand0_reg[4] ? 5'h12 : _operand0_rec_T_30; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_32 = operand0_reg[5] ? 5'h11 : _operand0_rec_T_31; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_33 = operand0_reg[6] ? 5'h10 : _operand0_rec_T_32; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_34 = operand0_reg[7] ? 5'hf : _operand0_rec_T_33; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_35 = operand0_reg[8] ? 5'he : _operand0_rec_T_34; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_36 = operand0_reg[9] ? 5'hd : _operand0_rec_T_35; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_37 = operand0_reg[10] ? 5'hc : _operand0_rec_T_36; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_38 = operand0_reg[11] ? 5'hb : _operand0_rec_T_37; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_39 = operand0_reg[12] ? 5'ha : _operand0_rec_T_38; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_40 = operand0_reg[13] ? 5'h9 : _operand0_rec_T_39; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_41 = operand0_reg[14] ? 5'h8 : _operand0_rec_T_40; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_42 = operand0_reg[15] ? 5'h7 : _operand0_rec_T_41; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_43 = operand0_reg[16] ? 5'h6 : _operand0_rec_T_42; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_44 = operand0_reg[17] ? 5'h5 : _operand0_rec_T_43; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_45 = operand0_reg[18] ? 5'h4 : _operand0_rec_T_44; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_46 = operand0_reg[19] ? 5'h3 : _operand0_rec_T_45; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_47 = operand0_reg[20] ? 5'h2 : _operand0_rec_T_46; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_48 = operand0_reg[21] ? 5'h1 : _operand0_rec_T_47; // @[Mux.scala 47:69]
  wire [4:0] _operand0_rec_T_49 = operand0_reg[22] ? 5'h0 : _operand0_rec_T_48; // @[Mux.scala 47:69]
  wire [53:0] _GEN_0 = {{31'd0}, operand0_reg[22:0]}; // @[rawFloatFromFN.scala 54:36]
  wire [53:0] _operand0_rec_T_50 = _GEN_0 << _operand0_rec_T_49; // @[rawFloatFromFN.scala 54:36]
  wire [22:0] _operand0_rec_T_52 = {_operand0_rec_T_50[21:0], 1'h0}; // @[rawFloatFromFN.scala 54:64]
  wire [8:0] _GEN_1 = {{4'd0}, _operand0_rec_T_49}; // @[rawFloatFromFN.scala 57:26]
  wire [8:0] _operand0_rec_T_53 = _GEN_1 ^ 9'h1ff; // @[rawFloatFromFN.scala 57:26]
  wire [8:0] _operand0_rec_T_54 = _operand0_rec_T_3 ? _operand0_rec_T_53 : {{1'd0}, operand0_reg[30:23]}; // @[rawFloatFromFN.scala 56:16]
  wire [1:0] _operand0_rec_T_55 = _operand0_rec_T_3 ? 2'h2 : 2'h1; // @[rawFloatFromFN.scala 60:27]
  wire [7:0] _GEN_2 = {{6'd0}, _operand0_rec_T_55}; // @[rawFloatFromFN.scala 60:22]
  wire [7:0] _operand0_rec_T_56 = 8'h80 | _GEN_2; // @[rawFloatFromFN.scala 60:22]
  wire [8:0] _GEN_3 = {{1'd0}, _operand0_rec_T_56}; // @[rawFloatFromFN.scala 59:15]
  wire [8:0] _operand0_rec_T_58 = _operand0_rec_T_54 + _GEN_3; // @[rawFloatFromFN.scala 59:15]
  wire  _operand0_rec_T_59 = _operand0_rec_T_3 & _operand0_rec_T_4; // @[rawFloatFromFN.scala 62:34]
  wire  _operand0_rec_T_61 = _operand0_rec_T_58[8:7] == 2'h3; // @[rawFloatFromFN.scala 63:62]
  wire  _operand0_rec_T_63 = _operand0_rec_T_61 & ~_operand0_rec_T_4; // @[rawFloatFromFN.scala 66:33]
  wire [9:0] _operand0_rec_T_66 = {1'b0,$signed(_operand0_rec_T_58)}; // @[rawFloatFromFN.scala 70:48]
  wire  operand0_rec_hi_lo = ~_operand0_rec_T_59; // @[rawFloatFromFN.scala 72:29]
  wire [22:0] operand0_rec_lo = _operand0_rec_T_3 ? _operand0_rec_T_52 : operand0_reg[22:0]; // @[rawFloatFromFN.scala 72:42]
  wire [24:0] _operand0_rec_T_67 = {1'h0,operand0_rec_hi_lo,operand0_rec_lo}; // @[Cat.scala 30:58]
  wire [2:0] _operand0_rec_T_69 = _operand0_rec_T_59 ? 3'h0 : _operand0_rec_T_66[8:6]; // @[recFNFromFN.scala 48:16]
  wire [2:0] _GEN_4 = {{2'd0}, _operand0_rec_T_63}; // @[recFNFromFN.scala 48:79]
  wire [2:0] operand0_rec_hi_lo_1 = _operand0_rec_T_69 | _GEN_4; // @[recFNFromFN.scala 48:79]
  wire [5:0] operand0_rec_lo_hi = _operand0_rec_T_66[5:0]; // @[recFNFromFN.scala 50:23]
  wire [22:0] operand0_rec_lo_lo = _operand0_rec_T_67[22:0]; // @[recFNFromFN.scala 51:22]
  wire [28:0] operand0_rec_lo_1 = {operand0_rec_lo_hi,operand0_rec_lo_lo}; // @[Cat.scala 30:58]
  wire [3:0] operand0_rec_hi_1 = {operand0_reg[31],operand0_rec_hi_lo_1}; // @[Cat.scala 30:58]
  wire  _operand1_rec_T_3 = operand1_reg[30:23] == 8'h0; // @[rawFloatFromFN.scala 50:34]
  wire  _operand1_rec_T_4 = operand1_reg[22:0] == 23'h0; // @[rawFloatFromFN.scala 51:38]
  wire [4:0] _operand1_rec_T_28 = operand1_reg[1] ? 5'h15 : 5'h16; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_29 = operand1_reg[2] ? 5'h14 : _operand1_rec_T_28; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_30 = operand1_reg[3] ? 5'h13 : _operand1_rec_T_29; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_31 = operand1_reg[4] ? 5'h12 : _operand1_rec_T_30; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_32 = operand1_reg[5] ? 5'h11 : _operand1_rec_T_31; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_33 = operand1_reg[6] ? 5'h10 : _operand1_rec_T_32; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_34 = operand1_reg[7] ? 5'hf : _operand1_rec_T_33; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_35 = operand1_reg[8] ? 5'he : _operand1_rec_T_34; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_36 = operand1_reg[9] ? 5'hd : _operand1_rec_T_35; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_37 = operand1_reg[10] ? 5'hc : _operand1_rec_T_36; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_38 = operand1_reg[11] ? 5'hb : _operand1_rec_T_37; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_39 = operand1_reg[12] ? 5'ha : _operand1_rec_T_38; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_40 = operand1_reg[13] ? 5'h9 : _operand1_rec_T_39; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_41 = operand1_reg[14] ? 5'h8 : _operand1_rec_T_40; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_42 = operand1_reg[15] ? 5'h7 : _operand1_rec_T_41; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_43 = operand1_reg[16] ? 5'h6 : _operand1_rec_T_42; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_44 = operand1_reg[17] ? 5'h5 : _operand1_rec_T_43; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_45 = operand1_reg[18] ? 5'h4 : _operand1_rec_T_44; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_46 = operand1_reg[19] ? 5'h3 : _operand1_rec_T_45; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_47 = operand1_reg[20] ? 5'h2 : _operand1_rec_T_46; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_48 = operand1_reg[21] ? 5'h1 : _operand1_rec_T_47; // @[Mux.scala 47:69]
  wire [4:0] _operand1_rec_T_49 = operand1_reg[22] ? 5'h0 : _operand1_rec_T_48; // @[Mux.scala 47:69]
  wire [53:0] _GEN_5 = {{31'd0}, operand1_reg[22:0]}; // @[rawFloatFromFN.scala 54:36]
  wire [53:0] _operand1_rec_T_50 = _GEN_5 << _operand1_rec_T_49; // @[rawFloatFromFN.scala 54:36]
  wire [22:0] _operand1_rec_T_52 = {_operand1_rec_T_50[21:0], 1'h0}; // @[rawFloatFromFN.scala 54:64]
  wire [8:0] _GEN_6 = {{4'd0}, _operand1_rec_T_49}; // @[rawFloatFromFN.scala 57:26]
  wire [8:0] _operand1_rec_T_53 = _GEN_6 ^ 9'h1ff; // @[rawFloatFromFN.scala 57:26]
  wire [8:0] _operand1_rec_T_54 = _operand1_rec_T_3 ? _operand1_rec_T_53 : {{1'd0}, operand1_reg[30:23]}; // @[rawFloatFromFN.scala 56:16]
  wire [1:0] _operand1_rec_T_55 = _operand1_rec_T_3 ? 2'h2 : 2'h1; // @[rawFloatFromFN.scala 60:27]
  wire [7:0] _GEN_7 = {{6'd0}, _operand1_rec_T_55}; // @[rawFloatFromFN.scala 60:22]
  wire [7:0] _operand1_rec_T_56 = 8'h80 | _GEN_7; // @[rawFloatFromFN.scala 60:22]
  wire [8:0] _GEN_8 = {{1'd0}, _operand1_rec_T_56}; // @[rawFloatFromFN.scala 59:15]
  wire [8:0] _operand1_rec_T_58 = _operand1_rec_T_54 + _GEN_8; // @[rawFloatFromFN.scala 59:15]
  wire  _operand1_rec_T_59 = _operand1_rec_T_3 & _operand1_rec_T_4; // @[rawFloatFromFN.scala 62:34]
  wire  _operand1_rec_T_61 = _operand1_rec_T_58[8:7] == 2'h3; // @[rawFloatFromFN.scala 63:62]
  wire  _operand1_rec_T_63 = _operand1_rec_T_61 & ~_operand1_rec_T_4; // @[rawFloatFromFN.scala 66:33]
  wire [9:0] _operand1_rec_T_66 = {1'b0,$signed(_operand1_rec_T_58)}; // @[rawFloatFromFN.scala 70:48]
  wire  operand1_rec_hi_lo = ~_operand1_rec_T_59; // @[rawFloatFromFN.scala 72:29]
  wire [22:0] operand1_rec_lo = _operand1_rec_T_3 ? _operand1_rec_T_52 : operand1_reg[22:0]; // @[rawFloatFromFN.scala 72:42]
  wire [24:0] _operand1_rec_T_67 = {1'h0,operand1_rec_hi_lo,operand1_rec_lo}; // @[Cat.scala 30:58]
  wire [2:0] _operand1_rec_T_69 = _operand1_rec_T_59 ? 3'h0 : _operand1_rec_T_66[8:6]; // @[recFNFromFN.scala 48:16]
  wire [2:0] _GEN_9 = {{2'd0}, _operand1_rec_T_63}; // @[recFNFromFN.scala 48:79]
  wire [2:0] operand1_rec_hi_lo_1 = _operand1_rec_T_69 | _GEN_9; // @[recFNFromFN.scala 48:79]
  wire [5:0] operand1_rec_lo_hi = _operand1_rec_T_66[5:0]; // @[recFNFromFN.scala 50:23]
  wire [22:0] operand1_rec_lo_lo = _operand1_rec_T_67[22:0]; // @[recFNFromFN.scala 51:22]
  wire [28:0] operand1_rec_lo_1 = {operand1_rec_lo_hi,operand1_rec_lo_lo}; // @[Cat.scala 30:58]
  wire [3:0] operand1_rec_hi_1 = {operand1_reg[31],operand1_rec_hi_lo_1}; // @[Cat.scala 30:58]
  wire  _output_T_2 = adder_io_out[31:29] == 3'h0; // @[rawFloatFromRecFN.scala 51:54]
  wire  _output_T_4 = adder_io_out[31:30] == 2'h3; // @[rawFloatFromRecFN.scala 52:54]
  wire  _output_T_6 = _output_T_4 & adder_io_out[29]; // @[rawFloatFromRecFN.scala 55:33]
  wire  _output_T_9 = _output_T_4 & ~adder_io_out[29]; // @[rawFloatFromRecFN.scala 56:33]
  wire [9:0] _output_T_11 = {1'b0,$signed(adder_io_out[31:23])}; // @[rawFloatFromRecFN.scala 59:27]
  wire  output_hi_lo = ~_output_T_2; // @[rawFloatFromRecFN.scala 60:39]
  wire [22:0] output_lo = adder_io_out[22:0]; // @[rawFloatFromRecFN.scala 60:51]
  wire [24:0] _output_T_12 = {1'h0,output_hi_lo,output_lo}; // @[Cat.scala 30:58]
  wire  _output_T_13 = $signed(_output_T_11) < 10'sh82; // @[fNFromRecFN.scala 50:39]
  wire [4:0] _output_T_16 = 5'h1 - _output_T_11[4:0]; // @[fNFromRecFN.scala 51:39]
  wire [23:0] _output_T_18 = _output_T_12[24:1] >> _output_T_16; // @[fNFromRecFN.scala 52:42]
  wire [7:0] _output_T_22 = _output_T_11[7:0] - 8'h81; // @[fNFromRecFN.scala 57:45]
  wire [7:0] _output_T_23 = _output_T_13 ? 8'h0 : _output_T_22; // @[fNFromRecFN.scala 55:16]
  wire  _output_T_24 = _output_T_6 | _output_T_9; // @[fNFromRecFN.scala 59:44]
  wire [7:0] _output_T_26 = _output_T_24 ? 8'hff : 8'h0; // @[Bitwise.scala 72:12]
  wire [7:0] output_hi_lo_1 = _output_T_23 | _output_T_26; // @[fNFromRecFN.scala 59:15]
  wire [22:0] _output_T_28 = _output_T_9 ? 23'h0 : _output_T_12[22:0]; // @[fNFromRecFN.scala 63:20]
  wire [22:0] output_lo_1 = _output_T_13 ? _output_T_18[22:0] : _output_T_28; // @[fNFromRecFN.scala 61:16]
  wire [8:0] output_hi_1 = {adder_io_out[32],output_hi_lo_1}; // @[Cat.scala 30:58]
  reg [31:0] shiftRegs_0; // @[fpunits.scala 94:26]
  reg [31:0] shiftRegs_1; // @[fpunits.scala 94:26]
  reg [31:0] shiftRegs_2; // @[fpunits.scala 94:26]
  AddRecFN adder ( // @[fpunits.scala 75:23]
    .io_a(adder_io_a),
    .io_b(adder_io_b),
    .io_out(adder_io_out)
  );
  assign result = shiftRegs_2; // @[fpunits.scala 100:14]
  assign adder_io_a = {operand0_rec_hi_1,operand0_rec_lo_1}; // @[Cat.scala 30:58]
  assign adder_io_b = {operand1_rec_hi_1,operand1_rec_lo_1}; // @[Cat.scala 30:58]
  always @(posedge new_clock) begin
    operand0_reg <= operand0; // @[fpunits.scala 67:31]
    operand1_reg <= operand1; // @[fpunits.scala 68:31]
    shiftRegs_0 <= {output_hi_1,output_lo_1}; // @[Cat.scala 30:58]
    shiftRegs_1 <= shiftRegs_0; // @[fpunits.scala 97:26]
    shiftRegs_2 <= shiftRegs_1; // @[fpunits.scala 97:26]
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  operand0_reg = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  operand1_reg = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  shiftRegs_0 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  shiftRegs_1 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  shiftRegs_2 = _RAND_4[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module DelayBuffer(
  input         clock,
  input         reset,
  input  [32:0] valid_in,
  input         ready_in,
  output [32:0] valid_out
);
`ifdef RANDOMIZE_REG_INIT
  reg [63:0] _RAND_0;
  reg [63:0] _RAND_1;
  reg [63:0] _RAND_2;
  reg [63:0] _RAND_3;
  reg [63:0] _RAND_4;
  reg [63:0] _RAND_5;
  reg [63:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  reg [32:0] shift_register_0; // @[elastic_component.scala 564:31]
  reg [32:0] shift_register_1; // @[elastic_component.scala 564:31]
  reg [32:0] shift_register_2; // @[elastic_component.scala 564:31]
  reg [32:0] shift_register_3; // @[elastic_component.scala 564:31]
  reg [32:0] shift_register_4; // @[elastic_component.scala 564:31]
  reg [32:0] shift_register_5; // @[elastic_component.scala 564:31]
  reg [32:0] shift_register_6; // @[elastic_component.scala 564:31]
  assign valid_out = shift_register_6; // @[elastic_component.scala 572:13]
  always @(posedge clock) begin
    if (reset) begin // @[elastic_component.scala 564:31]
      shift_register_0 <= 33'h0; // @[elastic_component.scala 564:31]
    end else if (ready_in) begin // @[elastic_component.scala 566:18]
      shift_register_0 <= valid_in; // @[elastic_component.scala 567:23]
    end
    if (reset) begin // @[elastic_component.scala 564:31]
      shift_register_1 <= 33'h0; // @[elastic_component.scala 564:31]
    end else if (ready_in) begin // @[elastic_component.scala 566:18]
      shift_register_1 <= shift_register_0; // @[elastic_component.scala 569:25]
    end
    if (reset) begin // @[elastic_component.scala 564:31]
      shift_register_2 <= 33'h0; // @[elastic_component.scala 564:31]
    end else if (ready_in) begin // @[elastic_component.scala 566:18]
      shift_register_2 <= shift_register_1; // @[elastic_component.scala 569:25]
    end
    if (reset) begin // @[elastic_component.scala 564:31]
      shift_register_3 <= 33'h0; // @[elastic_component.scala 564:31]
    end else if (ready_in) begin // @[elastic_component.scala 566:18]
      shift_register_3 <= shift_register_2; // @[elastic_component.scala 569:25]
    end
    if (reset) begin // @[elastic_component.scala 564:31]
      shift_register_4 <= 33'h0; // @[elastic_component.scala 564:31]
    end else if (ready_in) begin // @[elastic_component.scala 566:18]
      shift_register_4 <= shift_register_3; // @[elastic_component.scala 569:25]
    end
    if (reset) begin // @[elastic_component.scala 564:31]
      shift_register_5 <= 33'h0; // @[elastic_component.scala 564:31]
    end else if (ready_in) begin // @[elastic_component.scala 566:18]
      shift_register_5 <= shift_register_4; // @[elastic_component.scala 569:25]
    end
    if (reset) begin // @[elastic_component.scala 564:31]
      shift_register_6 <= 33'h0; // @[elastic_component.scala 564:31]
    end else if (ready_in) begin // @[elastic_component.scala 566:18]
      shift_register_6 <= shift_register_5; // @[elastic_component.scala 569:25]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {2{`RANDOM}};
  shift_register_0 = _RAND_0[32:0];
  _RAND_1 = {2{`RANDOM}};
  shift_register_1 = _RAND_1[32:0];
  _RAND_2 = {2{`RANDOM}};
  shift_register_2 = _RAND_2[32:0];
  _RAND_3 = {2{`RANDOM}};
  shift_register_3 = _RAND_3[32:0];
  _RAND_4 = {2{`RANDOM}};
  shift_register_4 = _RAND_4[32:0];
  _RAND_5 = {2{`RANDOM}};
  shift_register_5 = _RAND_5[32:0];
  _RAND_6 = {2{`RANDOM}};
  shift_register_6 = _RAND_6[32:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module DelayBuffer_1(
  input         clock,
  input         reset,
  input  [31:0] valid_in,
  input         ready_in,
  output [31:0] valid_out
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
`endif // RANDOMIZE_REG_INIT
  reg [31:0] shift_register_0; // @[elastic_component.scala 564:31]
  reg [31:0] shift_register_1; // @[elastic_component.scala 564:31]
  reg [31:0] shift_register_2; // @[elastic_component.scala 564:31]
  assign valid_out = shift_register_2; // @[elastic_component.scala 572:13]
  always @(posedge clock) begin
    if (reset) begin // @[elastic_component.scala 564:31]
      shift_register_0 <= 32'h0; // @[elastic_component.scala 564:31]
    end else if (ready_in) begin // @[elastic_component.scala 566:18]
      shift_register_0 <= valid_in; // @[elastic_component.scala 567:23]
    end
    if (reset) begin // @[elastic_component.scala 564:31]
      shift_register_1 <= 32'h0; // @[elastic_component.scala 564:31]
    end else if (ready_in) begin // @[elastic_component.scala 566:18]
      shift_register_1 <= shift_register_0; // @[elastic_component.scala 569:25]
    end
    if (reset) begin // @[elastic_component.scala 564:31]
      shift_register_2 <= 32'h0; // @[elastic_component.scala 564:31]
    end else if (ready_in) begin // @[elastic_component.scala 566:18]
      shift_register_2 <= shift_register_1; // @[elastic_component.scala 569:25]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  shift_register_0 = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  shift_register_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  shift_register_2 = _RAND_2[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h427de9e6; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42642907; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42025e7a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h418046b5; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42bfdc49; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h42bdc508; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42741e66; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_1(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4283bd90; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4293c27c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h40b83866; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41d67940; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h4289fa6a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h429e19bc; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42944e29; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_2(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42827c81; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42646be9; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42b922a4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h4209e296; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h427448be; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h428ab01c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h424873a3; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_3(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4285a3bd; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h41b11b58; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h41096169; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h420cd399; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h41beb0fe; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h428dc84f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42a3f7f0; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_4(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42855a5b; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42525c0f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h429fb6f2; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h4187a9ee; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42113fca; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h42098c37; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h4206877c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_5(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4284be69; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42869950; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42613ba9; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h4222b7dc; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h4296a5a5; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h40da16f5; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42bac4d5; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_6(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h426c921a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h429c6618; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h421a2a07; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42b20647; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42c4a6a0; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h42b5427e; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h41bb1184; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_7(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h426a06d1; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42b0a802; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h4229399b; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h4299914d; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h423dbf9f; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h42569176; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h429b24c6; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_line(
  input         clock,
  input         reset,
  output        var0_ready,
  input         var0_valid,
  input  [31:0] var0_bits,
  output        var1_ready,
  input         var1_valid,
  input  [31:0] var1_bits,
  output        var2_ready,
  input         var2_valid,
  input  [31:0] var2_bits,
  output        var3_ready,
  input         var3_valid,
  input  [31:0] var3_bits,
  output        var4_ready,
  input         var4_valid,
  input  [31:0] var4_bits,
  output        var5_ready,
  input         var5_valid,
  input  [31:0] var5_bits,
  output        var6_ready,
  input         var6_valid,
  input  [31:0] var6_bits,
  output        var7_ready,
  input         var7_valid,
  input  [31:0] var7_bits,
  output        var8_ready,
  input         var8_valid,
  input  [31:0] var8_bits,
  input         var9_ready,
  output        var9_valid,
  output [31:0] var9_bits,
  input         var10_ready,
  output        var10_valid,
  output [31:0] var10_bits,
  input         var11_ready,
  output        var11_valid,
  output [31:0] var11_bits,
  input         var12_ready,
  output        var12_valid,
  output [31:0] var12_bits,
  input         var13_ready,
  output        var13_valid,
  output [31:0] var13_bits,
  input         var14_ready,
  output        var14_valid,
  output [31:0] var14_bits,
  input         var15_ready,
  output        var15_valid,
  output [31:0] var15_bits,
  input         var16_ready,
  output        var16_valid,
  output [31:0] var16_bits
);
  wire  PE_0_clock; // @[systolic_array.scala 36:26]
  wire  PE_0_reset; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_1_clock; // @[systolic_array.scala 37:26]
  wire  PE_1_reset; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_2_clock; // @[systolic_array.scala 38:26]
  wire  PE_2_reset; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_3_clock; // @[systolic_array.scala 39:26]
  wire  PE_3_reset; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_4_clock; // @[systolic_array.scala 40:26]
  wire  PE_4_reset; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_5_clock; // @[systolic_array.scala 41:26]
  wire  PE_5_reset; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_6_clock; // @[systolic_array.scala 42:26]
  wire  PE_6_reset; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_7_clock; // @[systolic_array.scala 43:26]
  wire  PE_7_reset; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_A_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_out_bits; // @[systolic_array.scala 43:26]
  PE PE_0 ( // @[systolic_array.scala 36:26]
    .clock(PE_0_clock),
    .reset(PE_0_reset),
    .A_in_ready(PE_0_A_in_ready),
    .A_in_valid(PE_0_A_in_valid),
    .A_in_bits(PE_0_A_in_bits),
    .C_in_ready(PE_0_C_in_ready),
    .C_in_valid(PE_0_C_in_valid),
    .C_in_bits(PE_0_C_in_bits),
    .A_out_ready(PE_0_A_out_ready),
    .A_out_valid(PE_0_A_out_valid),
    .A_out_bits(PE_0_A_out_bits),
    .C_out_ready(PE_0_C_out_ready),
    .C_out_valid(PE_0_C_out_valid),
    .C_out_bits(PE_0_C_out_bits)
  );
  PE_1 PE_1 ( // @[systolic_array.scala 37:26]
    .clock(PE_1_clock),
    .reset(PE_1_reset),
    .A_in_ready(PE_1_A_in_ready),
    .A_in_valid(PE_1_A_in_valid),
    .A_in_bits(PE_1_A_in_bits),
    .C_in_ready(PE_1_C_in_ready),
    .C_in_valid(PE_1_C_in_valid),
    .C_in_bits(PE_1_C_in_bits),
    .A_out_ready(PE_1_A_out_ready),
    .A_out_valid(PE_1_A_out_valid),
    .A_out_bits(PE_1_A_out_bits),
    .C_out_ready(PE_1_C_out_ready),
    .C_out_valid(PE_1_C_out_valid),
    .C_out_bits(PE_1_C_out_bits)
  );
  PE_2 PE_2 ( // @[systolic_array.scala 38:26]
    .clock(PE_2_clock),
    .reset(PE_2_reset),
    .A_in_ready(PE_2_A_in_ready),
    .A_in_valid(PE_2_A_in_valid),
    .A_in_bits(PE_2_A_in_bits),
    .C_in_ready(PE_2_C_in_ready),
    .C_in_valid(PE_2_C_in_valid),
    .C_in_bits(PE_2_C_in_bits),
    .A_out_ready(PE_2_A_out_ready),
    .A_out_valid(PE_2_A_out_valid),
    .A_out_bits(PE_2_A_out_bits),
    .C_out_ready(PE_2_C_out_ready),
    .C_out_valid(PE_2_C_out_valid),
    .C_out_bits(PE_2_C_out_bits)
  );
  PE_3 PE_3 ( // @[systolic_array.scala 39:26]
    .clock(PE_3_clock),
    .reset(PE_3_reset),
    .A_in_ready(PE_3_A_in_ready),
    .A_in_valid(PE_3_A_in_valid),
    .A_in_bits(PE_3_A_in_bits),
    .C_in_ready(PE_3_C_in_ready),
    .C_in_valid(PE_3_C_in_valid),
    .C_in_bits(PE_3_C_in_bits),
    .A_out_ready(PE_3_A_out_ready),
    .A_out_valid(PE_3_A_out_valid),
    .A_out_bits(PE_3_A_out_bits),
    .C_out_ready(PE_3_C_out_ready),
    .C_out_valid(PE_3_C_out_valid),
    .C_out_bits(PE_3_C_out_bits)
  );
  PE_4 PE_4 ( // @[systolic_array.scala 40:26]
    .clock(PE_4_clock),
    .reset(PE_4_reset),
    .A_in_ready(PE_4_A_in_ready),
    .A_in_valid(PE_4_A_in_valid),
    .A_in_bits(PE_4_A_in_bits),
    .C_in_ready(PE_4_C_in_ready),
    .C_in_valid(PE_4_C_in_valid),
    .C_in_bits(PE_4_C_in_bits),
    .A_out_ready(PE_4_A_out_ready),
    .A_out_valid(PE_4_A_out_valid),
    .A_out_bits(PE_4_A_out_bits),
    .C_out_ready(PE_4_C_out_ready),
    .C_out_valid(PE_4_C_out_valid),
    .C_out_bits(PE_4_C_out_bits)
  );
  PE_5 PE_5 ( // @[systolic_array.scala 41:26]
    .clock(PE_5_clock),
    .reset(PE_5_reset),
    .A_in_ready(PE_5_A_in_ready),
    .A_in_valid(PE_5_A_in_valid),
    .A_in_bits(PE_5_A_in_bits),
    .C_in_ready(PE_5_C_in_ready),
    .C_in_valid(PE_5_C_in_valid),
    .C_in_bits(PE_5_C_in_bits),
    .A_out_ready(PE_5_A_out_ready),
    .A_out_valid(PE_5_A_out_valid),
    .A_out_bits(PE_5_A_out_bits),
    .C_out_ready(PE_5_C_out_ready),
    .C_out_valid(PE_5_C_out_valid),
    .C_out_bits(PE_5_C_out_bits)
  );
  PE_6 PE_6 ( // @[systolic_array.scala 42:26]
    .clock(PE_6_clock),
    .reset(PE_6_reset),
    .A_in_ready(PE_6_A_in_ready),
    .A_in_valid(PE_6_A_in_valid),
    .A_in_bits(PE_6_A_in_bits),
    .C_in_ready(PE_6_C_in_ready),
    .C_in_valid(PE_6_C_in_valid),
    .C_in_bits(PE_6_C_in_bits),
    .A_out_ready(PE_6_A_out_ready),
    .A_out_valid(PE_6_A_out_valid),
    .A_out_bits(PE_6_A_out_bits),
    .C_out_ready(PE_6_C_out_ready),
    .C_out_valid(PE_6_C_out_valid),
    .C_out_bits(PE_6_C_out_bits)
  );
  PE_7 PE_7 ( // @[systolic_array.scala 43:26]
    .clock(PE_7_clock),
    .reset(PE_7_reset),
    .A_in_ready(PE_7_A_in_ready),
    .A_in_valid(PE_7_A_in_valid),
    .A_in_bits(PE_7_A_in_bits),
    .C_in_ready(PE_7_C_in_ready),
    .C_in_valid(PE_7_C_in_valid),
    .C_in_bits(PE_7_C_in_bits),
    .C_out_ready(PE_7_C_out_ready),
    .C_out_valid(PE_7_C_out_valid),
    .C_out_bits(PE_7_C_out_bits)
  );
  assign var0_ready = PE_0_A_in_ready; // @[systolic_array.scala 45:19]
  assign var1_ready = PE_0_C_in_ready; // @[systolic_array.scala 46:19]
  assign var2_ready = PE_1_C_in_ready; // @[systolic_array.scala 49:19]
  assign var3_ready = PE_2_C_in_ready; // @[systolic_array.scala 52:19]
  assign var4_ready = PE_3_C_in_ready; // @[systolic_array.scala 55:19]
  assign var5_ready = PE_4_C_in_ready; // @[systolic_array.scala 58:19]
  assign var6_ready = PE_5_C_in_ready; // @[systolic_array.scala 61:19]
  assign var7_ready = PE_6_C_in_ready; // @[systolic_array.scala 64:19]
  assign var8_ready = PE_7_C_in_ready; // @[systolic_array.scala 67:19]
  assign var9_valid = PE_0_C_out_valid; // @[systolic_array.scala 47:14]
  assign var9_bits = PE_0_C_out_bits; // @[systolic_array.scala 47:14]
  assign var10_valid = PE_1_C_out_valid; // @[systolic_array.scala 50:15]
  assign var10_bits = PE_1_C_out_bits; // @[systolic_array.scala 50:15]
  assign var11_valid = PE_2_C_out_valid; // @[systolic_array.scala 53:15]
  assign var11_bits = PE_2_C_out_bits; // @[systolic_array.scala 53:15]
  assign var12_valid = PE_3_C_out_valid; // @[systolic_array.scala 56:15]
  assign var12_bits = PE_3_C_out_bits; // @[systolic_array.scala 56:15]
  assign var13_valid = PE_4_C_out_valid; // @[systolic_array.scala 59:15]
  assign var13_bits = PE_4_C_out_bits; // @[systolic_array.scala 59:15]
  assign var14_valid = PE_5_C_out_valid; // @[systolic_array.scala 62:15]
  assign var14_bits = PE_5_C_out_bits; // @[systolic_array.scala 62:15]
  assign var15_valid = PE_6_C_out_valid; // @[systolic_array.scala 65:15]
  assign var15_bits = PE_6_C_out_bits; // @[systolic_array.scala 65:15]
  assign var16_valid = PE_7_C_out_valid; // @[systolic_array.scala 68:15]
  assign var16_bits = PE_7_C_out_bits; // @[systolic_array.scala 68:15]
  assign PE_0_clock = clock;
  assign PE_0_reset = reset;
  assign PE_0_A_in_valid = var0_valid; // @[systolic_array.scala 45:19]
  assign PE_0_A_in_bits = var0_bits; // @[systolic_array.scala 45:19]
  assign PE_0_C_in_valid = var1_valid; // @[systolic_array.scala 46:19]
  assign PE_0_C_in_bits = var1_bits; // @[systolic_array.scala 46:19]
  assign PE_0_A_out_ready = PE_1_A_in_ready; // @[systolic_array.scala 48:19]
  assign PE_0_C_out_ready = var9_ready; // @[systolic_array.scala 47:14]
  assign PE_1_clock = clock;
  assign PE_1_reset = reset;
  assign PE_1_A_in_valid = PE_0_A_out_valid; // @[systolic_array.scala 48:19]
  assign PE_1_A_in_bits = PE_0_A_out_bits; // @[systolic_array.scala 48:19]
  assign PE_1_C_in_valid = var2_valid; // @[systolic_array.scala 49:19]
  assign PE_1_C_in_bits = var2_bits; // @[systolic_array.scala 49:19]
  assign PE_1_A_out_ready = PE_2_A_in_ready; // @[systolic_array.scala 51:19]
  assign PE_1_C_out_ready = var10_ready; // @[systolic_array.scala 50:15]
  assign PE_2_clock = clock;
  assign PE_2_reset = reset;
  assign PE_2_A_in_valid = PE_1_A_out_valid; // @[systolic_array.scala 51:19]
  assign PE_2_A_in_bits = PE_1_A_out_bits; // @[systolic_array.scala 51:19]
  assign PE_2_C_in_valid = var3_valid; // @[systolic_array.scala 52:19]
  assign PE_2_C_in_bits = var3_bits; // @[systolic_array.scala 52:19]
  assign PE_2_A_out_ready = PE_3_A_in_ready; // @[systolic_array.scala 54:19]
  assign PE_2_C_out_ready = var11_ready; // @[systolic_array.scala 53:15]
  assign PE_3_clock = clock;
  assign PE_3_reset = reset;
  assign PE_3_A_in_valid = PE_2_A_out_valid; // @[systolic_array.scala 54:19]
  assign PE_3_A_in_bits = PE_2_A_out_bits; // @[systolic_array.scala 54:19]
  assign PE_3_C_in_valid = var4_valid; // @[systolic_array.scala 55:19]
  assign PE_3_C_in_bits = var4_bits; // @[systolic_array.scala 55:19]
  assign PE_3_A_out_ready = PE_4_A_in_ready; // @[systolic_array.scala 57:19]
  assign PE_3_C_out_ready = var12_ready; // @[systolic_array.scala 56:15]
  assign PE_4_clock = clock;
  assign PE_4_reset = reset;
  assign PE_4_A_in_valid = PE_3_A_out_valid; // @[systolic_array.scala 57:19]
  assign PE_4_A_in_bits = PE_3_A_out_bits; // @[systolic_array.scala 57:19]
  assign PE_4_C_in_valid = var5_valid; // @[systolic_array.scala 58:19]
  assign PE_4_C_in_bits = var5_bits; // @[systolic_array.scala 58:19]
  assign PE_4_A_out_ready = PE_5_A_in_ready; // @[systolic_array.scala 60:19]
  assign PE_4_C_out_ready = var13_ready; // @[systolic_array.scala 59:15]
  assign PE_5_clock = clock;
  assign PE_5_reset = reset;
  assign PE_5_A_in_valid = PE_4_A_out_valid; // @[systolic_array.scala 60:19]
  assign PE_5_A_in_bits = PE_4_A_out_bits; // @[systolic_array.scala 60:19]
  assign PE_5_C_in_valid = var6_valid; // @[systolic_array.scala 61:19]
  assign PE_5_C_in_bits = var6_bits; // @[systolic_array.scala 61:19]
  assign PE_5_A_out_ready = PE_6_A_in_ready; // @[systolic_array.scala 63:19]
  assign PE_5_C_out_ready = var14_ready; // @[systolic_array.scala 62:15]
  assign PE_6_clock = clock;
  assign PE_6_reset = reset;
  assign PE_6_A_in_valid = PE_5_A_out_valid; // @[systolic_array.scala 63:19]
  assign PE_6_A_in_bits = PE_5_A_out_bits; // @[systolic_array.scala 63:19]
  assign PE_6_C_in_valid = var7_valid; // @[systolic_array.scala 64:19]
  assign PE_6_C_in_bits = var7_bits; // @[systolic_array.scala 64:19]
  assign PE_6_A_out_ready = PE_7_A_in_ready; // @[systolic_array.scala 66:19]
  assign PE_6_C_out_ready = var15_ready; // @[systolic_array.scala 65:15]
  assign PE_7_clock = clock;
  assign PE_7_reset = reset;
  assign PE_7_A_in_valid = PE_6_A_out_valid; // @[systolic_array.scala 66:19]
  assign PE_7_A_in_bits = PE_6_A_out_bits; // @[systolic_array.scala 66:19]
  assign PE_7_C_in_valid = var8_valid; // @[systolic_array.scala 67:19]
  assign PE_7_C_in_bits = var8_bits; // @[systolic_array.scala 67:19]
  assign PE_7_C_out_ready = var16_ready; // @[systolic_array.scala 68:15]
endmodule
module PE_8(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h427115ec; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42b0c973; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h40120685; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42be70ea; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h41493f37; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h41e0cf42; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42854f64; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_9(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h426ff061; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42452fab; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42334b7d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h426ae48d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42161e64; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h425c5ed3; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h428be702; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_10(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h426e9cf9; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4232dcef; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h429046a6; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41b62f68; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h423e8543; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h4236b4a8; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h4280c651; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_11(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h426ef8b4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42975df9; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h4025fd5c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41ad6db4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h429aaa5a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h4193eed0; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42a436ef; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_12(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h426de584; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42a58f75; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h4133a0dd; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41c7b2ce; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42b07331; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h3d3c9f80; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h41e7944a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_13(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h427635ad; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4224ee55; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h429b97f4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h421e45a8; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42654c99; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h41eadde4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42a019dd; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_14(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h427433fb; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h423f40ae; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h41d9f696; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41cd94d8; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h424bf621; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h3fcbf192; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42734662; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_15(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42734ea7; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h41a3264f; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h424c6cf2; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41e37918; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h429977c4; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h419c715a; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h4184268e; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_line_1(
  input         clock,
  input         reset,
  output        var0_ready,
  input         var0_valid,
  input  [31:0] var0_bits,
  output        var1_ready,
  input         var1_valid,
  input  [31:0] var1_bits,
  output        var2_ready,
  input         var2_valid,
  input  [31:0] var2_bits,
  output        var3_ready,
  input         var3_valid,
  input  [31:0] var3_bits,
  output        var4_ready,
  input         var4_valid,
  input  [31:0] var4_bits,
  output        var5_ready,
  input         var5_valid,
  input  [31:0] var5_bits,
  output        var6_ready,
  input         var6_valid,
  input  [31:0] var6_bits,
  output        var7_ready,
  input         var7_valid,
  input  [31:0] var7_bits,
  output        var8_ready,
  input         var8_valid,
  input  [31:0] var8_bits,
  input         var9_ready,
  output        var9_valid,
  output [31:0] var9_bits,
  input         var10_ready,
  output        var10_valid,
  output [31:0] var10_bits,
  input         var11_ready,
  output        var11_valid,
  output [31:0] var11_bits,
  input         var12_ready,
  output        var12_valid,
  output [31:0] var12_bits,
  input         var13_ready,
  output        var13_valid,
  output [31:0] var13_bits,
  input         var14_ready,
  output        var14_valid,
  output [31:0] var14_bits,
  input         var15_ready,
  output        var15_valid,
  output [31:0] var15_bits,
  input         var16_ready,
  output        var16_valid,
  output [31:0] var16_bits
);
  wire  PE_0_clock; // @[systolic_array.scala 36:26]
  wire  PE_0_reset; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_1_clock; // @[systolic_array.scala 37:26]
  wire  PE_1_reset; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_2_clock; // @[systolic_array.scala 38:26]
  wire  PE_2_reset; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_3_clock; // @[systolic_array.scala 39:26]
  wire  PE_3_reset; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_4_clock; // @[systolic_array.scala 40:26]
  wire  PE_4_reset; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_5_clock; // @[systolic_array.scala 41:26]
  wire  PE_5_reset; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_6_clock; // @[systolic_array.scala 42:26]
  wire  PE_6_reset; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_7_clock; // @[systolic_array.scala 43:26]
  wire  PE_7_reset; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_A_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_out_bits; // @[systolic_array.scala 43:26]
  PE_8 PE_0 ( // @[systolic_array.scala 36:26]
    .clock(PE_0_clock),
    .reset(PE_0_reset),
    .A_in_ready(PE_0_A_in_ready),
    .A_in_valid(PE_0_A_in_valid),
    .A_in_bits(PE_0_A_in_bits),
    .C_in_ready(PE_0_C_in_ready),
    .C_in_valid(PE_0_C_in_valid),
    .C_in_bits(PE_0_C_in_bits),
    .A_out_ready(PE_0_A_out_ready),
    .A_out_valid(PE_0_A_out_valid),
    .A_out_bits(PE_0_A_out_bits),
    .C_out_ready(PE_0_C_out_ready),
    .C_out_valid(PE_0_C_out_valid),
    .C_out_bits(PE_0_C_out_bits)
  );
  PE_9 PE_1 ( // @[systolic_array.scala 37:26]
    .clock(PE_1_clock),
    .reset(PE_1_reset),
    .A_in_ready(PE_1_A_in_ready),
    .A_in_valid(PE_1_A_in_valid),
    .A_in_bits(PE_1_A_in_bits),
    .C_in_ready(PE_1_C_in_ready),
    .C_in_valid(PE_1_C_in_valid),
    .C_in_bits(PE_1_C_in_bits),
    .A_out_ready(PE_1_A_out_ready),
    .A_out_valid(PE_1_A_out_valid),
    .A_out_bits(PE_1_A_out_bits),
    .C_out_ready(PE_1_C_out_ready),
    .C_out_valid(PE_1_C_out_valid),
    .C_out_bits(PE_1_C_out_bits)
  );
  PE_10 PE_2 ( // @[systolic_array.scala 38:26]
    .clock(PE_2_clock),
    .reset(PE_2_reset),
    .A_in_ready(PE_2_A_in_ready),
    .A_in_valid(PE_2_A_in_valid),
    .A_in_bits(PE_2_A_in_bits),
    .C_in_ready(PE_2_C_in_ready),
    .C_in_valid(PE_2_C_in_valid),
    .C_in_bits(PE_2_C_in_bits),
    .A_out_ready(PE_2_A_out_ready),
    .A_out_valid(PE_2_A_out_valid),
    .A_out_bits(PE_2_A_out_bits),
    .C_out_ready(PE_2_C_out_ready),
    .C_out_valid(PE_2_C_out_valid),
    .C_out_bits(PE_2_C_out_bits)
  );
  PE_11 PE_3 ( // @[systolic_array.scala 39:26]
    .clock(PE_3_clock),
    .reset(PE_3_reset),
    .A_in_ready(PE_3_A_in_ready),
    .A_in_valid(PE_3_A_in_valid),
    .A_in_bits(PE_3_A_in_bits),
    .C_in_ready(PE_3_C_in_ready),
    .C_in_valid(PE_3_C_in_valid),
    .C_in_bits(PE_3_C_in_bits),
    .A_out_ready(PE_3_A_out_ready),
    .A_out_valid(PE_3_A_out_valid),
    .A_out_bits(PE_3_A_out_bits),
    .C_out_ready(PE_3_C_out_ready),
    .C_out_valid(PE_3_C_out_valid),
    .C_out_bits(PE_3_C_out_bits)
  );
  PE_12 PE_4 ( // @[systolic_array.scala 40:26]
    .clock(PE_4_clock),
    .reset(PE_4_reset),
    .A_in_ready(PE_4_A_in_ready),
    .A_in_valid(PE_4_A_in_valid),
    .A_in_bits(PE_4_A_in_bits),
    .C_in_ready(PE_4_C_in_ready),
    .C_in_valid(PE_4_C_in_valid),
    .C_in_bits(PE_4_C_in_bits),
    .A_out_ready(PE_4_A_out_ready),
    .A_out_valid(PE_4_A_out_valid),
    .A_out_bits(PE_4_A_out_bits),
    .C_out_ready(PE_4_C_out_ready),
    .C_out_valid(PE_4_C_out_valid),
    .C_out_bits(PE_4_C_out_bits)
  );
  PE_13 PE_5 ( // @[systolic_array.scala 41:26]
    .clock(PE_5_clock),
    .reset(PE_5_reset),
    .A_in_ready(PE_5_A_in_ready),
    .A_in_valid(PE_5_A_in_valid),
    .A_in_bits(PE_5_A_in_bits),
    .C_in_ready(PE_5_C_in_ready),
    .C_in_valid(PE_5_C_in_valid),
    .C_in_bits(PE_5_C_in_bits),
    .A_out_ready(PE_5_A_out_ready),
    .A_out_valid(PE_5_A_out_valid),
    .A_out_bits(PE_5_A_out_bits),
    .C_out_ready(PE_5_C_out_ready),
    .C_out_valid(PE_5_C_out_valid),
    .C_out_bits(PE_5_C_out_bits)
  );
  PE_14 PE_6 ( // @[systolic_array.scala 42:26]
    .clock(PE_6_clock),
    .reset(PE_6_reset),
    .A_in_ready(PE_6_A_in_ready),
    .A_in_valid(PE_6_A_in_valid),
    .A_in_bits(PE_6_A_in_bits),
    .C_in_ready(PE_6_C_in_ready),
    .C_in_valid(PE_6_C_in_valid),
    .C_in_bits(PE_6_C_in_bits),
    .A_out_ready(PE_6_A_out_ready),
    .A_out_valid(PE_6_A_out_valid),
    .A_out_bits(PE_6_A_out_bits),
    .C_out_ready(PE_6_C_out_ready),
    .C_out_valid(PE_6_C_out_valid),
    .C_out_bits(PE_6_C_out_bits)
  );
  PE_15 PE_7 ( // @[systolic_array.scala 43:26]
    .clock(PE_7_clock),
    .reset(PE_7_reset),
    .A_in_ready(PE_7_A_in_ready),
    .A_in_valid(PE_7_A_in_valid),
    .A_in_bits(PE_7_A_in_bits),
    .C_in_ready(PE_7_C_in_ready),
    .C_in_valid(PE_7_C_in_valid),
    .C_in_bits(PE_7_C_in_bits),
    .C_out_ready(PE_7_C_out_ready),
    .C_out_valid(PE_7_C_out_valid),
    .C_out_bits(PE_7_C_out_bits)
  );
  assign var0_ready = PE_0_A_in_ready; // @[systolic_array.scala 45:19]
  assign var1_ready = PE_0_C_in_ready; // @[systolic_array.scala 46:19]
  assign var2_ready = PE_1_C_in_ready; // @[systolic_array.scala 49:19]
  assign var3_ready = PE_2_C_in_ready; // @[systolic_array.scala 52:19]
  assign var4_ready = PE_3_C_in_ready; // @[systolic_array.scala 55:19]
  assign var5_ready = PE_4_C_in_ready; // @[systolic_array.scala 58:19]
  assign var6_ready = PE_5_C_in_ready; // @[systolic_array.scala 61:19]
  assign var7_ready = PE_6_C_in_ready; // @[systolic_array.scala 64:19]
  assign var8_ready = PE_7_C_in_ready; // @[systolic_array.scala 67:19]
  assign var9_valid = PE_0_C_out_valid; // @[systolic_array.scala 47:14]
  assign var9_bits = PE_0_C_out_bits; // @[systolic_array.scala 47:14]
  assign var10_valid = PE_1_C_out_valid; // @[systolic_array.scala 50:15]
  assign var10_bits = PE_1_C_out_bits; // @[systolic_array.scala 50:15]
  assign var11_valid = PE_2_C_out_valid; // @[systolic_array.scala 53:15]
  assign var11_bits = PE_2_C_out_bits; // @[systolic_array.scala 53:15]
  assign var12_valid = PE_3_C_out_valid; // @[systolic_array.scala 56:15]
  assign var12_bits = PE_3_C_out_bits; // @[systolic_array.scala 56:15]
  assign var13_valid = PE_4_C_out_valid; // @[systolic_array.scala 59:15]
  assign var13_bits = PE_4_C_out_bits; // @[systolic_array.scala 59:15]
  assign var14_valid = PE_5_C_out_valid; // @[systolic_array.scala 62:15]
  assign var14_bits = PE_5_C_out_bits; // @[systolic_array.scala 62:15]
  assign var15_valid = PE_6_C_out_valid; // @[systolic_array.scala 65:15]
  assign var15_bits = PE_6_C_out_bits; // @[systolic_array.scala 65:15]
  assign var16_valid = PE_7_C_out_valid; // @[systolic_array.scala 68:15]
  assign var16_bits = PE_7_C_out_bits; // @[systolic_array.scala 68:15]
  assign PE_0_clock = clock;
  assign PE_0_reset = reset;
  assign PE_0_A_in_valid = var0_valid; // @[systolic_array.scala 45:19]
  assign PE_0_A_in_bits = var0_bits; // @[systolic_array.scala 45:19]
  assign PE_0_C_in_valid = var1_valid; // @[systolic_array.scala 46:19]
  assign PE_0_C_in_bits = var1_bits; // @[systolic_array.scala 46:19]
  assign PE_0_A_out_ready = PE_1_A_in_ready; // @[systolic_array.scala 48:19]
  assign PE_0_C_out_ready = var9_ready; // @[systolic_array.scala 47:14]
  assign PE_1_clock = clock;
  assign PE_1_reset = reset;
  assign PE_1_A_in_valid = PE_0_A_out_valid; // @[systolic_array.scala 48:19]
  assign PE_1_A_in_bits = PE_0_A_out_bits; // @[systolic_array.scala 48:19]
  assign PE_1_C_in_valid = var2_valid; // @[systolic_array.scala 49:19]
  assign PE_1_C_in_bits = var2_bits; // @[systolic_array.scala 49:19]
  assign PE_1_A_out_ready = PE_2_A_in_ready; // @[systolic_array.scala 51:19]
  assign PE_1_C_out_ready = var10_ready; // @[systolic_array.scala 50:15]
  assign PE_2_clock = clock;
  assign PE_2_reset = reset;
  assign PE_2_A_in_valid = PE_1_A_out_valid; // @[systolic_array.scala 51:19]
  assign PE_2_A_in_bits = PE_1_A_out_bits; // @[systolic_array.scala 51:19]
  assign PE_2_C_in_valid = var3_valid; // @[systolic_array.scala 52:19]
  assign PE_2_C_in_bits = var3_bits; // @[systolic_array.scala 52:19]
  assign PE_2_A_out_ready = PE_3_A_in_ready; // @[systolic_array.scala 54:19]
  assign PE_2_C_out_ready = var11_ready; // @[systolic_array.scala 53:15]
  assign PE_3_clock = clock;
  assign PE_3_reset = reset;
  assign PE_3_A_in_valid = PE_2_A_out_valid; // @[systolic_array.scala 54:19]
  assign PE_3_A_in_bits = PE_2_A_out_bits; // @[systolic_array.scala 54:19]
  assign PE_3_C_in_valid = var4_valid; // @[systolic_array.scala 55:19]
  assign PE_3_C_in_bits = var4_bits; // @[systolic_array.scala 55:19]
  assign PE_3_A_out_ready = PE_4_A_in_ready; // @[systolic_array.scala 57:19]
  assign PE_3_C_out_ready = var12_ready; // @[systolic_array.scala 56:15]
  assign PE_4_clock = clock;
  assign PE_4_reset = reset;
  assign PE_4_A_in_valid = PE_3_A_out_valid; // @[systolic_array.scala 57:19]
  assign PE_4_A_in_bits = PE_3_A_out_bits; // @[systolic_array.scala 57:19]
  assign PE_4_C_in_valid = var5_valid; // @[systolic_array.scala 58:19]
  assign PE_4_C_in_bits = var5_bits; // @[systolic_array.scala 58:19]
  assign PE_4_A_out_ready = PE_5_A_in_ready; // @[systolic_array.scala 60:19]
  assign PE_4_C_out_ready = var13_ready; // @[systolic_array.scala 59:15]
  assign PE_5_clock = clock;
  assign PE_5_reset = reset;
  assign PE_5_A_in_valid = PE_4_A_out_valid; // @[systolic_array.scala 60:19]
  assign PE_5_A_in_bits = PE_4_A_out_bits; // @[systolic_array.scala 60:19]
  assign PE_5_C_in_valid = var6_valid; // @[systolic_array.scala 61:19]
  assign PE_5_C_in_bits = var6_bits; // @[systolic_array.scala 61:19]
  assign PE_5_A_out_ready = PE_6_A_in_ready; // @[systolic_array.scala 63:19]
  assign PE_5_C_out_ready = var14_ready; // @[systolic_array.scala 62:15]
  assign PE_6_clock = clock;
  assign PE_6_reset = reset;
  assign PE_6_A_in_valid = PE_5_A_out_valid; // @[systolic_array.scala 63:19]
  assign PE_6_A_in_bits = PE_5_A_out_bits; // @[systolic_array.scala 63:19]
  assign PE_6_C_in_valid = var7_valid; // @[systolic_array.scala 64:19]
  assign PE_6_C_in_bits = var7_bits; // @[systolic_array.scala 64:19]
  assign PE_6_A_out_ready = PE_7_A_in_ready; // @[systolic_array.scala 66:19]
  assign PE_6_C_out_ready = var15_ready; // @[systolic_array.scala 65:15]
  assign PE_7_clock = clock;
  assign PE_7_reset = reset;
  assign PE_7_A_in_valid = PE_6_A_out_valid; // @[systolic_array.scala 66:19]
  assign PE_7_A_in_bits = PE_6_A_out_bits; // @[systolic_array.scala 66:19]
  assign PE_7_C_in_valid = var8_valid; // @[systolic_array.scala 67:19]
  assign PE_7_C_in_bits = var8_bits; // @[systolic_array.scala 67:19]
  assign PE_7_C_out_ready = var16_ready; // @[systolic_array.scala 68:15]
endmodule
module PE_16(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4271f213; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h422add52; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42bec00d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42912fe4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h422f74dc; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h42a5b71d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42b25daf; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_17(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h427a4b6a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h429487eb; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h4233f671; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h40e2df97; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h425fbc9e; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h421dce4a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42783b78; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_18(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h427a8273; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h414d89c6; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h422d0e5d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41d376e0; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h415c1788; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h4048fefb; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42b9614e; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_19(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4278ae9f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42a9d225; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h40fe1825; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41403f1d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h41b22e45; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h413c87ac; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42a4a2f1; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_20(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4277b6f3; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4134c244; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42836baf; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h4295f10a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42784d73; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h42965d3f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42bcf2de; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_21(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42781bda; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4175cab4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h429e6c47; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42bb8893; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h420afe6f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h42968ec2; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h4207dd80; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_22(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42772d5a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h416d883a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h41a0371d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h429939ad; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h418c2c92; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h4283eb2f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h4287c9f0; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_23(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4259cb62; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4296f9a6; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h41b556da; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h4284bc94; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42a3d216; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h42be3a75; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h416db898; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_line_2(
  input         clock,
  input         reset,
  output        var0_ready,
  input         var0_valid,
  input  [31:0] var0_bits,
  output        var1_ready,
  input         var1_valid,
  input  [31:0] var1_bits,
  output        var2_ready,
  input         var2_valid,
  input  [31:0] var2_bits,
  output        var3_ready,
  input         var3_valid,
  input  [31:0] var3_bits,
  output        var4_ready,
  input         var4_valid,
  input  [31:0] var4_bits,
  output        var5_ready,
  input         var5_valid,
  input  [31:0] var5_bits,
  output        var6_ready,
  input         var6_valid,
  input  [31:0] var6_bits,
  output        var7_ready,
  input         var7_valid,
  input  [31:0] var7_bits,
  output        var8_ready,
  input         var8_valid,
  input  [31:0] var8_bits,
  input         var9_ready,
  output        var9_valid,
  output [31:0] var9_bits,
  input         var10_ready,
  output        var10_valid,
  output [31:0] var10_bits,
  input         var11_ready,
  output        var11_valid,
  output [31:0] var11_bits,
  input         var12_ready,
  output        var12_valid,
  output [31:0] var12_bits,
  input         var13_ready,
  output        var13_valid,
  output [31:0] var13_bits,
  input         var14_ready,
  output        var14_valid,
  output [31:0] var14_bits,
  input         var15_ready,
  output        var15_valid,
  output [31:0] var15_bits,
  input         var16_ready,
  output        var16_valid,
  output [31:0] var16_bits
);
  wire  PE_0_clock; // @[systolic_array.scala 36:26]
  wire  PE_0_reset; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_1_clock; // @[systolic_array.scala 37:26]
  wire  PE_1_reset; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_2_clock; // @[systolic_array.scala 38:26]
  wire  PE_2_reset; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_3_clock; // @[systolic_array.scala 39:26]
  wire  PE_3_reset; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_4_clock; // @[systolic_array.scala 40:26]
  wire  PE_4_reset; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_5_clock; // @[systolic_array.scala 41:26]
  wire  PE_5_reset; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_6_clock; // @[systolic_array.scala 42:26]
  wire  PE_6_reset; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_7_clock; // @[systolic_array.scala 43:26]
  wire  PE_7_reset; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_A_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_out_bits; // @[systolic_array.scala 43:26]
  PE_16 PE_0 ( // @[systolic_array.scala 36:26]
    .clock(PE_0_clock),
    .reset(PE_0_reset),
    .A_in_ready(PE_0_A_in_ready),
    .A_in_valid(PE_0_A_in_valid),
    .A_in_bits(PE_0_A_in_bits),
    .C_in_ready(PE_0_C_in_ready),
    .C_in_valid(PE_0_C_in_valid),
    .C_in_bits(PE_0_C_in_bits),
    .A_out_ready(PE_0_A_out_ready),
    .A_out_valid(PE_0_A_out_valid),
    .A_out_bits(PE_0_A_out_bits),
    .C_out_ready(PE_0_C_out_ready),
    .C_out_valid(PE_0_C_out_valid),
    .C_out_bits(PE_0_C_out_bits)
  );
  PE_17 PE_1 ( // @[systolic_array.scala 37:26]
    .clock(PE_1_clock),
    .reset(PE_1_reset),
    .A_in_ready(PE_1_A_in_ready),
    .A_in_valid(PE_1_A_in_valid),
    .A_in_bits(PE_1_A_in_bits),
    .C_in_ready(PE_1_C_in_ready),
    .C_in_valid(PE_1_C_in_valid),
    .C_in_bits(PE_1_C_in_bits),
    .A_out_ready(PE_1_A_out_ready),
    .A_out_valid(PE_1_A_out_valid),
    .A_out_bits(PE_1_A_out_bits),
    .C_out_ready(PE_1_C_out_ready),
    .C_out_valid(PE_1_C_out_valid),
    .C_out_bits(PE_1_C_out_bits)
  );
  PE_18 PE_2 ( // @[systolic_array.scala 38:26]
    .clock(PE_2_clock),
    .reset(PE_2_reset),
    .A_in_ready(PE_2_A_in_ready),
    .A_in_valid(PE_2_A_in_valid),
    .A_in_bits(PE_2_A_in_bits),
    .C_in_ready(PE_2_C_in_ready),
    .C_in_valid(PE_2_C_in_valid),
    .C_in_bits(PE_2_C_in_bits),
    .A_out_ready(PE_2_A_out_ready),
    .A_out_valid(PE_2_A_out_valid),
    .A_out_bits(PE_2_A_out_bits),
    .C_out_ready(PE_2_C_out_ready),
    .C_out_valid(PE_2_C_out_valid),
    .C_out_bits(PE_2_C_out_bits)
  );
  PE_19 PE_3 ( // @[systolic_array.scala 39:26]
    .clock(PE_3_clock),
    .reset(PE_3_reset),
    .A_in_ready(PE_3_A_in_ready),
    .A_in_valid(PE_3_A_in_valid),
    .A_in_bits(PE_3_A_in_bits),
    .C_in_ready(PE_3_C_in_ready),
    .C_in_valid(PE_3_C_in_valid),
    .C_in_bits(PE_3_C_in_bits),
    .A_out_ready(PE_3_A_out_ready),
    .A_out_valid(PE_3_A_out_valid),
    .A_out_bits(PE_3_A_out_bits),
    .C_out_ready(PE_3_C_out_ready),
    .C_out_valid(PE_3_C_out_valid),
    .C_out_bits(PE_3_C_out_bits)
  );
  PE_20 PE_4 ( // @[systolic_array.scala 40:26]
    .clock(PE_4_clock),
    .reset(PE_4_reset),
    .A_in_ready(PE_4_A_in_ready),
    .A_in_valid(PE_4_A_in_valid),
    .A_in_bits(PE_4_A_in_bits),
    .C_in_ready(PE_4_C_in_ready),
    .C_in_valid(PE_4_C_in_valid),
    .C_in_bits(PE_4_C_in_bits),
    .A_out_ready(PE_4_A_out_ready),
    .A_out_valid(PE_4_A_out_valid),
    .A_out_bits(PE_4_A_out_bits),
    .C_out_ready(PE_4_C_out_ready),
    .C_out_valid(PE_4_C_out_valid),
    .C_out_bits(PE_4_C_out_bits)
  );
  PE_21 PE_5 ( // @[systolic_array.scala 41:26]
    .clock(PE_5_clock),
    .reset(PE_5_reset),
    .A_in_ready(PE_5_A_in_ready),
    .A_in_valid(PE_5_A_in_valid),
    .A_in_bits(PE_5_A_in_bits),
    .C_in_ready(PE_5_C_in_ready),
    .C_in_valid(PE_5_C_in_valid),
    .C_in_bits(PE_5_C_in_bits),
    .A_out_ready(PE_5_A_out_ready),
    .A_out_valid(PE_5_A_out_valid),
    .A_out_bits(PE_5_A_out_bits),
    .C_out_ready(PE_5_C_out_ready),
    .C_out_valid(PE_5_C_out_valid),
    .C_out_bits(PE_5_C_out_bits)
  );
  PE_22 PE_6 ( // @[systolic_array.scala 42:26]
    .clock(PE_6_clock),
    .reset(PE_6_reset),
    .A_in_ready(PE_6_A_in_ready),
    .A_in_valid(PE_6_A_in_valid),
    .A_in_bits(PE_6_A_in_bits),
    .C_in_ready(PE_6_C_in_ready),
    .C_in_valid(PE_6_C_in_valid),
    .C_in_bits(PE_6_C_in_bits),
    .A_out_ready(PE_6_A_out_ready),
    .A_out_valid(PE_6_A_out_valid),
    .A_out_bits(PE_6_A_out_bits),
    .C_out_ready(PE_6_C_out_ready),
    .C_out_valid(PE_6_C_out_valid),
    .C_out_bits(PE_6_C_out_bits)
  );
  PE_23 PE_7 ( // @[systolic_array.scala 43:26]
    .clock(PE_7_clock),
    .reset(PE_7_reset),
    .A_in_ready(PE_7_A_in_ready),
    .A_in_valid(PE_7_A_in_valid),
    .A_in_bits(PE_7_A_in_bits),
    .C_in_ready(PE_7_C_in_ready),
    .C_in_valid(PE_7_C_in_valid),
    .C_in_bits(PE_7_C_in_bits),
    .C_out_ready(PE_7_C_out_ready),
    .C_out_valid(PE_7_C_out_valid),
    .C_out_bits(PE_7_C_out_bits)
  );
  assign var0_ready = PE_0_A_in_ready; // @[systolic_array.scala 45:19]
  assign var1_ready = PE_0_C_in_ready; // @[systolic_array.scala 46:19]
  assign var2_ready = PE_1_C_in_ready; // @[systolic_array.scala 49:19]
  assign var3_ready = PE_2_C_in_ready; // @[systolic_array.scala 52:19]
  assign var4_ready = PE_3_C_in_ready; // @[systolic_array.scala 55:19]
  assign var5_ready = PE_4_C_in_ready; // @[systolic_array.scala 58:19]
  assign var6_ready = PE_5_C_in_ready; // @[systolic_array.scala 61:19]
  assign var7_ready = PE_6_C_in_ready; // @[systolic_array.scala 64:19]
  assign var8_ready = PE_7_C_in_ready; // @[systolic_array.scala 67:19]
  assign var9_valid = PE_0_C_out_valid; // @[systolic_array.scala 47:14]
  assign var9_bits = PE_0_C_out_bits; // @[systolic_array.scala 47:14]
  assign var10_valid = PE_1_C_out_valid; // @[systolic_array.scala 50:15]
  assign var10_bits = PE_1_C_out_bits; // @[systolic_array.scala 50:15]
  assign var11_valid = PE_2_C_out_valid; // @[systolic_array.scala 53:15]
  assign var11_bits = PE_2_C_out_bits; // @[systolic_array.scala 53:15]
  assign var12_valid = PE_3_C_out_valid; // @[systolic_array.scala 56:15]
  assign var12_bits = PE_3_C_out_bits; // @[systolic_array.scala 56:15]
  assign var13_valid = PE_4_C_out_valid; // @[systolic_array.scala 59:15]
  assign var13_bits = PE_4_C_out_bits; // @[systolic_array.scala 59:15]
  assign var14_valid = PE_5_C_out_valid; // @[systolic_array.scala 62:15]
  assign var14_bits = PE_5_C_out_bits; // @[systolic_array.scala 62:15]
  assign var15_valid = PE_6_C_out_valid; // @[systolic_array.scala 65:15]
  assign var15_bits = PE_6_C_out_bits; // @[systolic_array.scala 65:15]
  assign var16_valid = PE_7_C_out_valid; // @[systolic_array.scala 68:15]
  assign var16_bits = PE_7_C_out_bits; // @[systolic_array.scala 68:15]
  assign PE_0_clock = clock;
  assign PE_0_reset = reset;
  assign PE_0_A_in_valid = var0_valid; // @[systolic_array.scala 45:19]
  assign PE_0_A_in_bits = var0_bits; // @[systolic_array.scala 45:19]
  assign PE_0_C_in_valid = var1_valid; // @[systolic_array.scala 46:19]
  assign PE_0_C_in_bits = var1_bits; // @[systolic_array.scala 46:19]
  assign PE_0_A_out_ready = PE_1_A_in_ready; // @[systolic_array.scala 48:19]
  assign PE_0_C_out_ready = var9_ready; // @[systolic_array.scala 47:14]
  assign PE_1_clock = clock;
  assign PE_1_reset = reset;
  assign PE_1_A_in_valid = PE_0_A_out_valid; // @[systolic_array.scala 48:19]
  assign PE_1_A_in_bits = PE_0_A_out_bits; // @[systolic_array.scala 48:19]
  assign PE_1_C_in_valid = var2_valid; // @[systolic_array.scala 49:19]
  assign PE_1_C_in_bits = var2_bits; // @[systolic_array.scala 49:19]
  assign PE_1_A_out_ready = PE_2_A_in_ready; // @[systolic_array.scala 51:19]
  assign PE_1_C_out_ready = var10_ready; // @[systolic_array.scala 50:15]
  assign PE_2_clock = clock;
  assign PE_2_reset = reset;
  assign PE_2_A_in_valid = PE_1_A_out_valid; // @[systolic_array.scala 51:19]
  assign PE_2_A_in_bits = PE_1_A_out_bits; // @[systolic_array.scala 51:19]
  assign PE_2_C_in_valid = var3_valid; // @[systolic_array.scala 52:19]
  assign PE_2_C_in_bits = var3_bits; // @[systolic_array.scala 52:19]
  assign PE_2_A_out_ready = PE_3_A_in_ready; // @[systolic_array.scala 54:19]
  assign PE_2_C_out_ready = var11_ready; // @[systolic_array.scala 53:15]
  assign PE_3_clock = clock;
  assign PE_3_reset = reset;
  assign PE_3_A_in_valid = PE_2_A_out_valid; // @[systolic_array.scala 54:19]
  assign PE_3_A_in_bits = PE_2_A_out_bits; // @[systolic_array.scala 54:19]
  assign PE_3_C_in_valid = var4_valid; // @[systolic_array.scala 55:19]
  assign PE_3_C_in_bits = var4_bits; // @[systolic_array.scala 55:19]
  assign PE_3_A_out_ready = PE_4_A_in_ready; // @[systolic_array.scala 57:19]
  assign PE_3_C_out_ready = var12_ready; // @[systolic_array.scala 56:15]
  assign PE_4_clock = clock;
  assign PE_4_reset = reset;
  assign PE_4_A_in_valid = PE_3_A_out_valid; // @[systolic_array.scala 57:19]
  assign PE_4_A_in_bits = PE_3_A_out_bits; // @[systolic_array.scala 57:19]
  assign PE_4_C_in_valid = var5_valid; // @[systolic_array.scala 58:19]
  assign PE_4_C_in_bits = var5_bits; // @[systolic_array.scala 58:19]
  assign PE_4_A_out_ready = PE_5_A_in_ready; // @[systolic_array.scala 60:19]
  assign PE_4_C_out_ready = var13_ready; // @[systolic_array.scala 59:15]
  assign PE_5_clock = clock;
  assign PE_5_reset = reset;
  assign PE_5_A_in_valid = PE_4_A_out_valid; // @[systolic_array.scala 60:19]
  assign PE_5_A_in_bits = PE_4_A_out_bits; // @[systolic_array.scala 60:19]
  assign PE_5_C_in_valid = var6_valid; // @[systolic_array.scala 61:19]
  assign PE_5_C_in_bits = var6_bits; // @[systolic_array.scala 61:19]
  assign PE_5_A_out_ready = PE_6_A_in_ready; // @[systolic_array.scala 63:19]
  assign PE_5_C_out_ready = var14_ready; // @[systolic_array.scala 62:15]
  assign PE_6_clock = clock;
  assign PE_6_reset = reset;
  assign PE_6_A_in_valid = PE_5_A_out_valid; // @[systolic_array.scala 63:19]
  assign PE_6_A_in_bits = PE_5_A_out_bits; // @[systolic_array.scala 63:19]
  assign PE_6_C_in_valid = var7_valid; // @[systolic_array.scala 64:19]
  assign PE_6_C_in_bits = var7_bits; // @[systolic_array.scala 64:19]
  assign PE_6_A_out_ready = PE_7_A_in_ready; // @[systolic_array.scala 66:19]
  assign PE_6_C_out_ready = var15_ready; // @[systolic_array.scala 65:15]
  assign PE_7_clock = clock;
  assign PE_7_reset = reset;
  assign PE_7_A_in_valid = PE_6_A_out_valid; // @[systolic_array.scala 66:19]
  assign PE_7_A_in_bits = PE_6_A_out_bits; // @[systolic_array.scala 66:19]
  assign PE_7_C_in_valid = var8_valid; // @[systolic_array.scala 67:19]
  assign PE_7_C_in_bits = var8_bits; // @[systolic_array.scala 67:19]
  assign PE_7_C_out_ready = var16_ready; // @[systolic_array.scala 68:15]
endmodule
module PE_24(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42590ac0; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4221d22e; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h429d6bd9; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h4242aaf1; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h429f11e0; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h41b19e47; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42669525; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_25(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42596fa7; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4232144a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42b86c71; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h4286ed01; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h4250d4bc; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h41b26453; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42c24674; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_26(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42575b9c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h409d0382; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h4297237c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h415cfbc7; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h426fbb2e; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h421fe236; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42097284; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_27(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4256ad53; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h41808b08; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42c36845; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h420f9090; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h422dfdd8; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h41acf9fc; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h41a58d1e; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_28(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42571b67; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42ba755d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42bc8032; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h429527c5; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h426c905f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h4244004a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42a3ea6c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_29(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h425e6ab8; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h409f1a84; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h420e2dc4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h4200fe2d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h41c896b5; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h4166e184; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h41bb8f81; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_30(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h425ed8cc; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42a44444; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42005d9d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h428dde94; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h4222dde2; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h42273bad; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42a96b05; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_31(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h425d29a8; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h423a56c8; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h41e799c5; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42105127; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h41a8eaa0; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h426e4ec2; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h4275b36a; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_line_3(
  input         clock,
  input         reset,
  output        var0_ready,
  input         var0_valid,
  input  [31:0] var0_bits,
  output        var1_ready,
  input         var1_valid,
  input  [31:0] var1_bits,
  output        var2_ready,
  input         var2_valid,
  input  [31:0] var2_bits,
  output        var3_ready,
  input         var3_valid,
  input  [31:0] var3_bits,
  output        var4_ready,
  input         var4_valid,
  input  [31:0] var4_bits,
  output        var5_ready,
  input         var5_valid,
  input  [31:0] var5_bits,
  output        var6_ready,
  input         var6_valid,
  input  [31:0] var6_bits,
  output        var7_ready,
  input         var7_valid,
  input  [31:0] var7_bits,
  output        var8_ready,
  input         var8_valid,
  input  [31:0] var8_bits,
  input         var9_ready,
  output        var9_valid,
  output [31:0] var9_bits,
  input         var10_ready,
  output        var10_valid,
  output [31:0] var10_bits,
  input         var11_ready,
  output        var11_valid,
  output [31:0] var11_bits,
  input         var12_ready,
  output        var12_valid,
  output [31:0] var12_bits,
  input         var13_ready,
  output        var13_valid,
  output [31:0] var13_bits,
  input         var14_ready,
  output        var14_valid,
  output [31:0] var14_bits,
  input         var15_ready,
  output        var15_valid,
  output [31:0] var15_bits,
  input         var16_ready,
  output        var16_valid,
  output [31:0] var16_bits
);
  wire  PE_0_clock; // @[systolic_array.scala 36:26]
  wire  PE_0_reset; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_1_clock; // @[systolic_array.scala 37:26]
  wire  PE_1_reset; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_2_clock; // @[systolic_array.scala 38:26]
  wire  PE_2_reset; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_3_clock; // @[systolic_array.scala 39:26]
  wire  PE_3_reset; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_4_clock; // @[systolic_array.scala 40:26]
  wire  PE_4_reset; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_5_clock; // @[systolic_array.scala 41:26]
  wire  PE_5_reset; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_6_clock; // @[systolic_array.scala 42:26]
  wire  PE_6_reset; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_7_clock; // @[systolic_array.scala 43:26]
  wire  PE_7_reset; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_A_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_out_bits; // @[systolic_array.scala 43:26]
  PE_24 PE_0 ( // @[systolic_array.scala 36:26]
    .clock(PE_0_clock),
    .reset(PE_0_reset),
    .A_in_ready(PE_0_A_in_ready),
    .A_in_valid(PE_0_A_in_valid),
    .A_in_bits(PE_0_A_in_bits),
    .C_in_ready(PE_0_C_in_ready),
    .C_in_valid(PE_0_C_in_valid),
    .C_in_bits(PE_0_C_in_bits),
    .A_out_ready(PE_0_A_out_ready),
    .A_out_valid(PE_0_A_out_valid),
    .A_out_bits(PE_0_A_out_bits),
    .C_out_ready(PE_0_C_out_ready),
    .C_out_valid(PE_0_C_out_valid),
    .C_out_bits(PE_0_C_out_bits)
  );
  PE_25 PE_1 ( // @[systolic_array.scala 37:26]
    .clock(PE_1_clock),
    .reset(PE_1_reset),
    .A_in_ready(PE_1_A_in_ready),
    .A_in_valid(PE_1_A_in_valid),
    .A_in_bits(PE_1_A_in_bits),
    .C_in_ready(PE_1_C_in_ready),
    .C_in_valid(PE_1_C_in_valid),
    .C_in_bits(PE_1_C_in_bits),
    .A_out_ready(PE_1_A_out_ready),
    .A_out_valid(PE_1_A_out_valid),
    .A_out_bits(PE_1_A_out_bits),
    .C_out_ready(PE_1_C_out_ready),
    .C_out_valid(PE_1_C_out_valid),
    .C_out_bits(PE_1_C_out_bits)
  );
  PE_26 PE_2 ( // @[systolic_array.scala 38:26]
    .clock(PE_2_clock),
    .reset(PE_2_reset),
    .A_in_ready(PE_2_A_in_ready),
    .A_in_valid(PE_2_A_in_valid),
    .A_in_bits(PE_2_A_in_bits),
    .C_in_ready(PE_2_C_in_ready),
    .C_in_valid(PE_2_C_in_valid),
    .C_in_bits(PE_2_C_in_bits),
    .A_out_ready(PE_2_A_out_ready),
    .A_out_valid(PE_2_A_out_valid),
    .A_out_bits(PE_2_A_out_bits),
    .C_out_ready(PE_2_C_out_ready),
    .C_out_valid(PE_2_C_out_valid),
    .C_out_bits(PE_2_C_out_bits)
  );
  PE_27 PE_3 ( // @[systolic_array.scala 39:26]
    .clock(PE_3_clock),
    .reset(PE_3_reset),
    .A_in_ready(PE_3_A_in_ready),
    .A_in_valid(PE_3_A_in_valid),
    .A_in_bits(PE_3_A_in_bits),
    .C_in_ready(PE_3_C_in_ready),
    .C_in_valid(PE_3_C_in_valid),
    .C_in_bits(PE_3_C_in_bits),
    .A_out_ready(PE_3_A_out_ready),
    .A_out_valid(PE_3_A_out_valid),
    .A_out_bits(PE_3_A_out_bits),
    .C_out_ready(PE_3_C_out_ready),
    .C_out_valid(PE_3_C_out_valid),
    .C_out_bits(PE_3_C_out_bits)
  );
  PE_28 PE_4 ( // @[systolic_array.scala 40:26]
    .clock(PE_4_clock),
    .reset(PE_4_reset),
    .A_in_ready(PE_4_A_in_ready),
    .A_in_valid(PE_4_A_in_valid),
    .A_in_bits(PE_4_A_in_bits),
    .C_in_ready(PE_4_C_in_ready),
    .C_in_valid(PE_4_C_in_valid),
    .C_in_bits(PE_4_C_in_bits),
    .A_out_ready(PE_4_A_out_ready),
    .A_out_valid(PE_4_A_out_valid),
    .A_out_bits(PE_4_A_out_bits),
    .C_out_ready(PE_4_C_out_ready),
    .C_out_valid(PE_4_C_out_valid),
    .C_out_bits(PE_4_C_out_bits)
  );
  PE_29 PE_5 ( // @[systolic_array.scala 41:26]
    .clock(PE_5_clock),
    .reset(PE_5_reset),
    .A_in_ready(PE_5_A_in_ready),
    .A_in_valid(PE_5_A_in_valid),
    .A_in_bits(PE_5_A_in_bits),
    .C_in_ready(PE_5_C_in_ready),
    .C_in_valid(PE_5_C_in_valid),
    .C_in_bits(PE_5_C_in_bits),
    .A_out_ready(PE_5_A_out_ready),
    .A_out_valid(PE_5_A_out_valid),
    .A_out_bits(PE_5_A_out_bits),
    .C_out_ready(PE_5_C_out_ready),
    .C_out_valid(PE_5_C_out_valid),
    .C_out_bits(PE_5_C_out_bits)
  );
  PE_30 PE_6 ( // @[systolic_array.scala 42:26]
    .clock(PE_6_clock),
    .reset(PE_6_reset),
    .A_in_ready(PE_6_A_in_ready),
    .A_in_valid(PE_6_A_in_valid),
    .A_in_bits(PE_6_A_in_bits),
    .C_in_ready(PE_6_C_in_ready),
    .C_in_valid(PE_6_C_in_valid),
    .C_in_bits(PE_6_C_in_bits),
    .A_out_ready(PE_6_A_out_ready),
    .A_out_valid(PE_6_A_out_valid),
    .A_out_bits(PE_6_A_out_bits),
    .C_out_ready(PE_6_C_out_ready),
    .C_out_valid(PE_6_C_out_valid),
    .C_out_bits(PE_6_C_out_bits)
  );
  PE_31 PE_7 ( // @[systolic_array.scala 43:26]
    .clock(PE_7_clock),
    .reset(PE_7_reset),
    .A_in_ready(PE_7_A_in_ready),
    .A_in_valid(PE_7_A_in_valid),
    .A_in_bits(PE_7_A_in_bits),
    .C_in_ready(PE_7_C_in_ready),
    .C_in_valid(PE_7_C_in_valid),
    .C_in_bits(PE_7_C_in_bits),
    .C_out_ready(PE_7_C_out_ready),
    .C_out_valid(PE_7_C_out_valid),
    .C_out_bits(PE_7_C_out_bits)
  );
  assign var0_ready = PE_0_A_in_ready; // @[systolic_array.scala 45:19]
  assign var1_ready = PE_0_C_in_ready; // @[systolic_array.scala 46:19]
  assign var2_ready = PE_1_C_in_ready; // @[systolic_array.scala 49:19]
  assign var3_ready = PE_2_C_in_ready; // @[systolic_array.scala 52:19]
  assign var4_ready = PE_3_C_in_ready; // @[systolic_array.scala 55:19]
  assign var5_ready = PE_4_C_in_ready; // @[systolic_array.scala 58:19]
  assign var6_ready = PE_5_C_in_ready; // @[systolic_array.scala 61:19]
  assign var7_ready = PE_6_C_in_ready; // @[systolic_array.scala 64:19]
  assign var8_ready = PE_7_C_in_ready; // @[systolic_array.scala 67:19]
  assign var9_valid = PE_0_C_out_valid; // @[systolic_array.scala 47:14]
  assign var9_bits = PE_0_C_out_bits; // @[systolic_array.scala 47:14]
  assign var10_valid = PE_1_C_out_valid; // @[systolic_array.scala 50:15]
  assign var10_bits = PE_1_C_out_bits; // @[systolic_array.scala 50:15]
  assign var11_valid = PE_2_C_out_valid; // @[systolic_array.scala 53:15]
  assign var11_bits = PE_2_C_out_bits; // @[systolic_array.scala 53:15]
  assign var12_valid = PE_3_C_out_valid; // @[systolic_array.scala 56:15]
  assign var12_bits = PE_3_C_out_bits; // @[systolic_array.scala 56:15]
  assign var13_valid = PE_4_C_out_valid; // @[systolic_array.scala 59:15]
  assign var13_bits = PE_4_C_out_bits; // @[systolic_array.scala 59:15]
  assign var14_valid = PE_5_C_out_valid; // @[systolic_array.scala 62:15]
  assign var14_bits = PE_5_C_out_bits; // @[systolic_array.scala 62:15]
  assign var15_valid = PE_6_C_out_valid; // @[systolic_array.scala 65:15]
  assign var15_bits = PE_6_C_out_bits; // @[systolic_array.scala 65:15]
  assign var16_valid = PE_7_C_out_valid; // @[systolic_array.scala 68:15]
  assign var16_bits = PE_7_C_out_bits; // @[systolic_array.scala 68:15]
  assign PE_0_clock = clock;
  assign PE_0_reset = reset;
  assign PE_0_A_in_valid = var0_valid; // @[systolic_array.scala 45:19]
  assign PE_0_A_in_bits = var0_bits; // @[systolic_array.scala 45:19]
  assign PE_0_C_in_valid = var1_valid; // @[systolic_array.scala 46:19]
  assign PE_0_C_in_bits = var1_bits; // @[systolic_array.scala 46:19]
  assign PE_0_A_out_ready = PE_1_A_in_ready; // @[systolic_array.scala 48:19]
  assign PE_0_C_out_ready = var9_ready; // @[systolic_array.scala 47:14]
  assign PE_1_clock = clock;
  assign PE_1_reset = reset;
  assign PE_1_A_in_valid = PE_0_A_out_valid; // @[systolic_array.scala 48:19]
  assign PE_1_A_in_bits = PE_0_A_out_bits; // @[systolic_array.scala 48:19]
  assign PE_1_C_in_valid = var2_valid; // @[systolic_array.scala 49:19]
  assign PE_1_C_in_bits = var2_bits; // @[systolic_array.scala 49:19]
  assign PE_1_A_out_ready = PE_2_A_in_ready; // @[systolic_array.scala 51:19]
  assign PE_1_C_out_ready = var10_ready; // @[systolic_array.scala 50:15]
  assign PE_2_clock = clock;
  assign PE_2_reset = reset;
  assign PE_2_A_in_valid = PE_1_A_out_valid; // @[systolic_array.scala 51:19]
  assign PE_2_A_in_bits = PE_1_A_out_bits; // @[systolic_array.scala 51:19]
  assign PE_2_C_in_valid = var3_valid; // @[systolic_array.scala 52:19]
  assign PE_2_C_in_bits = var3_bits; // @[systolic_array.scala 52:19]
  assign PE_2_A_out_ready = PE_3_A_in_ready; // @[systolic_array.scala 54:19]
  assign PE_2_C_out_ready = var11_ready; // @[systolic_array.scala 53:15]
  assign PE_3_clock = clock;
  assign PE_3_reset = reset;
  assign PE_3_A_in_valid = PE_2_A_out_valid; // @[systolic_array.scala 54:19]
  assign PE_3_A_in_bits = PE_2_A_out_bits; // @[systolic_array.scala 54:19]
  assign PE_3_C_in_valid = var4_valid; // @[systolic_array.scala 55:19]
  assign PE_3_C_in_bits = var4_bits; // @[systolic_array.scala 55:19]
  assign PE_3_A_out_ready = PE_4_A_in_ready; // @[systolic_array.scala 57:19]
  assign PE_3_C_out_ready = var12_ready; // @[systolic_array.scala 56:15]
  assign PE_4_clock = clock;
  assign PE_4_reset = reset;
  assign PE_4_A_in_valid = PE_3_A_out_valid; // @[systolic_array.scala 57:19]
  assign PE_4_A_in_bits = PE_3_A_out_bits; // @[systolic_array.scala 57:19]
  assign PE_4_C_in_valid = var5_valid; // @[systolic_array.scala 58:19]
  assign PE_4_C_in_bits = var5_bits; // @[systolic_array.scala 58:19]
  assign PE_4_A_out_ready = PE_5_A_in_ready; // @[systolic_array.scala 60:19]
  assign PE_4_C_out_ready = var13_ready; // @[systolic_array.scala 59:15]
  assign PE_5_clock = clock;
  assign PE_5_reset = reset;
  assign PE_5_A_in_valid = PE_4_A_out_valid; // @[systolic_array.scala 60:19]
  assign PE_5_A_in_bits = PE_4_A_out_bits; // @[systolic_array.scala 60:19]
  assign PE_5_C_in_valid = var6_valid; // @[systolic_array.scala 61:19]
  assign PE_5_C_in_bits = var6_bits; // @[systolic_array.scala 61:19]
  assign PE_5_A_out_ready = PE_6_A_in_ready; // @[systolic_array.scala 63:19]
  assign PE_5_C_out_ready = var14_ready; // @[systolic_array.scala 62:15]
  assign PE_6_clock = clock;
  assign PE_6_reset = reset;
  assign PE_6_A_in_valid = PE_5_A_out_valid; // @[systolic_array.scala 63:19]
  assign PE_6_A_in_bits = PE_5_A_out_bits; // @[systolic_array.scala 63:19]
  assign PE_6_C_in_valid = var7_valid; // @[systolic_array.scala 64:19]
  assign PE_6_C_in_bits = var7_bits; // @[systolic_array.scala 64:19]
  assign PE_6_A_out_ready = PE_7_A_in_ready; // @[systolic_array.scala 66:19]
  assign PE_6_C_out_ready = var15_ready; // @[systolic_array.scala 65:15]
  assign PE_7_clock = clock;
  assign PE_7_reset = reset;
  assign PE_7_A_in_valid = PE_6_A_out_valid; // @[systolic_array.scala 66:19]
  assign PE_7_A_in_bits = PE_6_A_out_bits; // @[systolic_array.scala 66:19]
  assign PE_7_C_in_valid = var8_valid; // @[systolic_array.scala 67:19]
  assign PE_7_C_in_bits = var8_bits; // @[systolic_array.scala 67:19]
  assign PE_7_C_out_ready = var16_ready; // @[systolic_array.scala 68:15]
endmodule
module PE_32(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h425c1fa4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h41d639b6; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h41a507da; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h426d039f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h4295f445; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h4288d364; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h41f04c6e; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_33(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h425cc4c1; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h422a14ac; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h41772ec2; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h418a442e; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h4241c454; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h426debbc; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h41af774d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_34(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h425acc3b; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h41b19441; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h423fc5bb; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41ba6d8d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42aa27b4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h426bfc9c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h41c3c9a5; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_35(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h425b715a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4217c1f0; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h422b0d80; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42a2aa9f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h426a2b33; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h4248418f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h4182f483; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_36(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4263ef61; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h427780dc; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h41c47cea; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42bec889; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h4229f924; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h417c8ae1; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h4298aa3b; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_37(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42620008; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h416a65a4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42206727; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41aa6b55; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h41b0cb86; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h422a53e0; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h3f250a00; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_38(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h426289a1; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h41319fb4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42ab917a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h419d48c5; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42854e75; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h424f3800; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h41d9cc0b; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_39(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4260e3ab; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42429947; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42836071; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h40964268; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42b40af3; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h42c1b5ad; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h41cdb3d4; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_line_4(
  input         clock,
  input         reset,
  output        var0_ready,
  input         var0_valid,
  input  [31:0] var0_bits,
  output        var1_ready,
  input         var1_valid,
  input  [31:0] var1_bits,
  output        var2_ready,
  input         var2_valid,
  input  [31:0] var2_bits,
  output        var3_ready,
  input         var3_valid,
  input  [31:0] var3_bits,
  output        var4_ready,
  input         var4_valid,
  input  [31:0] var4_bits,
  output        var5_ready,
  input         var5_valid,
  input  [31:0] var5_bits,
  output        var6_ready,
  input         var6_valid,
  input  [31:0] var6_bits,
  output        var7_ready,
  input         var7_valid,
  input  [31:0] var7_bits,
  output        var8_ready,
  input         var8_valid,
  input  [31:0] var8_bits,
  input         var9_ready,
  output        var9_valid,
  output [31:0] var9_bits,
  input         var10_ready,
  output        var10_valid,
  output [31:0] var10_bits,
  input         var11_ready,
  output        var11_valid,
  output [31:0] var11_bits,
  input         var12_ready,
  output        var12_valid,
  output [31:0] var12_bits,
  input         var13_ready,
  output        var13_valid,
  output [31:0] var13_bits,
  input         var14_ready,
  output        var14_valid,
  output [31:0] var14_bits,
  input         var15_ready,
  output        var15_valid,
  output [31:0] var15_bits,
  input         var16_ready,
  output        var16_valid,
  output [31:0] var16_bits
);
  wire  PE_0_clock; // @[systolic_array.scala 36:26]
  wire  PE_0_reset; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_1_clock; // @[systolic_array.scala 37:26]
  wire  PE_1_reset; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_2_clock; // @[systolic_array.scala 38:26]
  wire  PE_2_reset; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_3_clock; // @[systolic_array.scala 39:26]
  wire  PE_3_reset; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_4_clock; // @[systolic_array.scala 40:26]
  wire  PE_4_reset; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_5_clock; // @[systolic_array.scala 41:26]
  wire  PE_5_reset; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_6_clock; // @[systolic_array.scala 42:26]
  wire  PE_6_reset; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_7_clock; // @[systolic_array.scala 43:26]
  wire  PE_7_reset; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_A_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_out_bits; // @[systolic_array.scala 43:26]
  PE_32 PE_0 ( // @[systolic_array.scala 36:26]
    .clock(PE_0_clock),
    .reset(PE_0_reset),
    .A_in_ready(PE_0_A_in_ready),
    .A_in_valid(PE_0_A_in_valid),
    .A_in_bits(PE_0_A_in_bits),
    .C_in_ready(PE_0_C_in_ready),
    .C_in_valid(PE_0_C_in_valid),
    .C_in_bits(PE_0_C_in_bits),
    .A_out_ready(PE_0_A_out_ready),
    .A_out_valid(PE_0_A_out_valid),
    .A_out_bits(PE_0_A_out_bits),
    .C_out_ready(PE_0_C_out_ready),
    .C_out_valid(PE_0_C_out_valid),
    .C_out_bits(PE_0_C_out_bits)
  );
  PE_33 PE_1 ( // @[systolic_array.scala 37:26]
    .clock(PE_1_clock),
    .reset(PE_1_reset),
    .A_in_ready(PE_1_A_in_ready),
    .A_in_valid(PE_1_A_in_valid),
    .A_in_bits(PE_1_A_in_bits),
    .C_in_ready(PE_1_C_in_ready),
    .C_in_valid(PE_1_C_in_valid),
    .C_in_bits(PE_1_C_in_bits),
    .A_out_ready(PE_1_A_out_ready),
    .A_out_valid(PE_1_A_out_valid),
    .A_out_bits(PE_1_A_out_bits),
    .C_out_ready(PE_1_C_out_ready),
    .C_out_valid(PE_1_C_out_valid),
    .C_out_bits(PE_1_C_out_bits)
  );
  PE_34 PE_2 ( // @[systolic_array.scala 38:26]
    .clock(PE_2_clock),
    .reset(PE_2_reset),
    .A_in_ready(PE_2_A_in_ready),
    .A_in_valid(PE_2_A_in_valid),
    .A_in_bits(PE_2_A_in_bits),
    .C_in_ready(PE_2_C_in_ready),
    .C_in_valid(PE_2_C_in_valid),
    .C_in_bits(PE_2_C_in_bits),
    .A_out_ready(PE_2_A_out_ready),
    .A_out_valid(PE_2_A_out_valid),
    .A_out_bits(PE_2_A_out_bits),
    .C_out_ready(PE_2_C_out_ready),
    .C_out_valid(PE_2_C_out_valid),
    .C_out_bits(PE_2_C_out_bits)
  );
  PE_35 PE_3 ( // @[systolic_array.scala 39:26]
    .clock(PE_3_clock),
    .reset(PE_3_reset),
    .A_in_ready(PE_3_A_in_ready),
    .A_in_valid(PE_3_A_in_valid),
    .A_in_bits(PE_3_A_in_bits),
    .C_in_ready(PE_3_C_in_ready),
    .C_in_valid(PE_3_C_in_valid),
    .C_in_bits(PE_3_C_in_bits),
    .A_out_ready(PE_3_A_out_ready),
    .A_out_valid(PE_3_A_out_valid),
    .A_out_bits(PE_3_A_out_bits),
    .C_out_ready(PE_3_C_out_ready),
    .C_out_valid(PE_3_C_out_valid),
    .C_out_bits(PE_3_C_out_bits)
  );
  PE_36 PE_4 ( // @[systolic_array.scala 40:26]
    .clock(PE_4_clock),
    .reset(PE_4_reset),
    .A_in_ready(PE_4_A_in_ready),
    .A_in_valid(PE_4_A_in_valid),
    .A_in_bits(PE_4_A_in_bits),
    .C_in_ready(PE_4_C_in_ready),
    .C_in_valid(PE_4_C_in_valid),
    .C_in_bits(PE_4_C_in_bits),
    .A_out_ready(PE_4_A_out_ready),
    .A_out_valid(PE_4_A_out_valid),
    .A_out_bits(PE_4_A_out_bits),
    .C_out_ready(PE_4_C_out_ready),
    .C_out_valid(PE_4_C_out_valid),
    .C_out_bits(PE_4_C_out_bits)
  );
  PE_37 PE_5 ( // @[systolic_array.scala 41:26]
    .clock(PE_5_clock),
    .reset(PE_5_reset),
    .A_in_ready(PE_5_A_in_ready),
    .A_in_valid(PE_5_A_in_valid),
    .A_in_bits(PE_5_A_in_bits),
    .C_in_ready(PE_5_C_in_ready),
    .C_in_valid(PE_5_C_in_valid),
    .C_in_bits(PE_5_C_in_bits),
    .A_out_ready(PE_5_A_out_ready),
    .A_out_valid(PE_5_A_out_valid),
    .A_out_bits(PE_5_A_out_bits),
    .C_out_ready(PE_5_C_out_ready),
    .C_out_valid(PE_5_C_out_valid),
    .C_out_bits(PE_5_C_out_bits)
  );
  PE_38 PE_6 ( // @[systolic_array.scala 42:26]
    .clock(PE_6_clock),
    .reset(PE_6_reset),
    .A_in_ready(PE_6_A_in_ready),
    .A_in_valid(PE_6_A_in_valid),
    .A_in_bits(PE_6_A_in_bits),
    .C_in_ready(PE_6_C_in_ready),
    .C_in_valid(PE_6_C_in_valid),
    .C_in_bits(PE_6_C_in_bits),
    .A_out_ready(PE_6_A_out_ready),
    .A_out_valid(PE_6_A_out_valid),
    .A_out_bits(PE_6_A_out_bits),
    .C_out_ready(PE_6_C_out_ready),
    .C_out_valid(PE_6_C_out_valid),
    .C_out_bits(PE_6_C_out_bits)
  );
  PE_39 PE_7 ( // @[systolic_array.scala 43:26]
    .clock(PE_7_clock),
    .reset(PE_7_reset),
    .A_in_ready(PE_7_A_in_ready),
    .A_in_valid(PE_7_A_in_valid),
    .A_in_bits(PE_7_A_in_bits),
    .C_in_ready(PE_7_C_in_ready),
    .C_in_valid(PE_7_C_in_valid),
    .C_in_bits(PE_7_C_in_bits),
    .C_out_ready(PE_7_C_out_ready),
    .C_out_valid(PE_7_C_out_valid),
    .C_out_bits(PE_7_C_out_bits)
  );
  assign var0_ready = PE_0_A_in_ready; // @[systolic_array.scala 45:19]
  assign var1_ready = PE_0_C_in_ready; // @[systolic_array.scala 46:19]
  assign var2_ready = PE_1_C_in_ready; // @[systolic_array.scala 49:19]
  assign var3_ready = PE_2_C_in_ready; // @[systolic_array.scala 52:19]
  assign var4_ready = PE_3_C_in_ready; // @[systolic_array.scala 55:19]
  assign var5_ready = PE_4_C_in_ready; // @[systolic_array.scala 58:19]
  assign var6_ready = PE_5_C_in_ready; // @[systolic_array.scala 61:19]
  assign var7_ready = PE_6_C_in_ready; // @[systolic_array.scala 64:19]
  assign var8_ready = PE_7_C_in_ready; // @[systolic_array.scala 67:19]
  assign var9_valid = PE_0_C_out_valid; // @[systolic_array.scala 47:14]
  assign var9_bits = PE_0_C_out_bits; // @[systolic_array.scala 47:14]
  assign var10_valid = PE_1_C_out_valid; // @[systolic_array.scala 50:15]
  assign var10_bits = PE_1_C_out_bits; // @[systolic_array.scala 50:15]
  assign var11_valid = PE_2_C_out_valid; // @[systolic_array.scala 53:15]
  assign var11_bits = PE_2_C_out_bits; // @[systolic_array.scala 53:15]
  assign var12_valid = PE_3_C_out_valid; // @[systolic_array.scala 56:15]
  assign var12_bits = PE_3_C_out_bits; // @[systolic_array.scala 56:15]
  assign var13_valid = PE_4_C_out_valid; // @[systolic_array.scala 59:15]
  assign var13_bits = PE_4_C_out_bits; // @[systolic_array.scala 59:15]
  assign var14_valid = PE_5_C_out_valid; // @[systolic_array.scala 62:15]
  assign var14_bits = PE_5_C_out_bits; // @[systolic_array.scala 62:15]
  assign var15_valid = PE_6_C_out_valid; // @[systolic_array.scala 65:15]
  assign var15_bits = PE_6_C_out_bits; // @[systolic_array.scala 65:15]
  assign var16_valid = PE_7_C_out_valid; // @[systolic_array.scala 68:15]
  assign var16_bits = PE_7_C_out_bits; // @[systolic_array.scala 68:15]
  assign PE_0_clock = clock;
  assign PE_0_reset = reset;
  assign PE_0_A_in_valid = var0_valid; // @[systolic_array.scala 45:19]
  assign PE_0_A_in_bits = var0_bits; // @[systolic_array.scala 45:19]
  assign PE_0_C_in_valid = var1_valid; // @[systolic_array.scala 46:19]
  assign PE_0_C_in_bits = var1_bits; // @[systolic_array.scala 46:19]
  assign PE_0_A_out_ready = PE_1_A_in_ready; // @[systolic_array.scala 48:19]
  assign PE_0_C_out_ready = var9_ready; // @[systolic_array.scala 47:14]
  assign PE_1_clock = clock;
  assign PE_1_reset = reset;
  assign PE_1_A_in_valid = PE_0_A_out_valid; // @[systolic_array.scala 48:19]
  assign PE_1_A_in_bits = PE_0_A_out_bits; // @[systolic_array.scala 48:19]
  assign PE_1_C_in_valid = var2_valid; // @[systolic_array.scala 49:19]
  assign PE_1_C_in_bits = var2_bits; // @[systolic_array.scala 49:19]
  assign PE_1_A_out_ready = PE_2_A_in_ready; // @[systolic_array.scala 51:19]
  assign PE_1_C_out_ready = var10_ready; // @[systolic_array.scala 50:15]
  assign PE_2_clock = clock;
  assign PE_2_reset = reset;
  assign PE_2_A_in_valid = PE_1_A_out_valid; // @[systolic_array.scala 51:19]
  assign PE_2_A_in_bits = PE_1_A_out_bits; // @[systolic_array.scala 51:19]
  assign PE_2_C_in_valid = var3_valid; // @[systolic_array.scala 52:19]
  assign PE_2_C_in_bits = var3_bits; // @[systolic_array.scala 52:19]
  assign PE_2_A_out_ready = PE_3_A_in_ready; // @[systolic_array.scala 54:19]
  assign PE_2_C_out_ready = var11_ready; // @[systolic_array.scala 53:15]
  assign PE_3_clock = clock;
  assign PE_3_reset = reset;
  assign PE_3_A_in_valid = PE_2_A_out_valid; // @[systolic_array.scala 54:19]
  assign PE_3_A_in_bits = PE_2_A_out_bits; // @[systolic_array.scala 54:19]
  assign PE_3_C_in_valid = var4_valid; // @[systolic_array.scala 55:19]
  assign PE_3_C_in_bits = var4_bits; // @[systolic_array.scala 55:19]
  assign PE_3_A_out_ready = PE_4_A_in_ready; // @[systolic_array.scala 57:19]
  assign PE_3_C_out_ready = var12_ready; // @[systolic_array.scala 56:15]
  assign PE_4_clock = clock;
  assign PE_4_reset = reset;
  assign PE_4_A_in_valid = PE_3_A_out_valid; // @[systolic_array.scala 57:19]
  assign PE_4_A_in_bits = PE_3_A_out_bits; // @[systolic_array.scala 57:19]
  assign PE_4_C_in_valid = var5_valid; // @[systolic_array.scala 58:19]
  assign PE_4_C_in_bits = var5_bits; // @[systolic_array.scala 58:19]
  assign PE_4_A_out_ready = PE_5_A_in_ready; // @[systolic_array.scala 60:19]
  assign PE_4_C_out_ready = var13_ready; // @[systolic_array.scala 59:15]
  assign PE_5_clock = clock;
  assign PE_5_reset = reset;
  assign PE_5_A_in_valid = PE_4_A_out_valid; // @[systolic_array.scala 60:19]
  assign PE_5_A_in_bits = PE_4_A_out_bits; // @[systolic_array.scala 60:19]
  assign PE_5_C_in_valid = var6_valid; // @[systolic_array.scala 61:19]
  assign PE_5_C_in_bits = var6_bits; // @[systolic_array.scala 61:19]
  assign PE_5_A_out_ready = PE_6_A_in_ready; // @[systolic_array.scala 63:19]
  assign PE_5_C_out_ready = var14_ready; // @[systolic_array.scala 62:15]
  assign PE_6_clock = clock;
  assign PE_6_reset = reset;
  assign PE_6_A_in_valid = PE_5_A_out_valid; // @[systolic_array.scala 63:19]
  assign PE_6_A_in_bits = PE_5_A_out_bits; // @[systolic_array.scala 63:19]
  assign PE_6_C_in_valid = var7_valid; // @[systolic_array.scala 64:19]
  assign PE_6_C_in_bits = var7_bits; // @[systolic_array.scala 64:19]
  assign PE_6_A_out_ready = PE_7_A_in_ready; // @[systolic_array.scala 66:19]
  assign PE_6_C_out_ready = var15_ready; // @[systolic_array.scala 65:15]
  assign PE_7_clock = clock;
  assign PE_7_reset = reset;
  assign PE_7_A_in_valid = PE_6_A_out_valid; // @[systolic_array.scala 66:19]
  assign PE_7_A_in_bits = PE_6_A_out_bits; // @[systolic_array.scala 66:19]
  assign PE_7_C_in_valid = var8_valid; // @[systolic_array.scala 67:19]
  assign PE_7_C_in_bits = var8_bits; // @[systolic_array.scala 67:19]
  assign PE_7_C_out_ready = var16_ready; // @[systolic_array.scala 68:15]
endmodule
module PE_40(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h425fd9a5; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h41e6beb4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h426577ec; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41def58a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h422f891e; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h40b61afe; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42bca65b; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_41(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4260633e; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h41ca5bbc; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h40433bb0; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41d1d2fe; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42b0e024; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h416e9e03; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h41a73d29; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_42(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4267f2c6; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4242dc29; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h41cd4f60; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41b90f13; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h425ca612; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h428ea0c2; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h416cbc9d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_43(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42687c5f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4234aaad; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h428eb1bf; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41abec83; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42c76e9d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h42a112d2; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42258103; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_44(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42669632; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h428e132a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h428bdd6c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42869562; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h41b256cc; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h420a7710; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42aeeb9f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_45(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h426756d4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h40c23b96; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h415e3a48; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42a9fc7e; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h41c557a4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h40f0e6b4; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h4232b03f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_46(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42659e85; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42c2d94d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h41dd9e60; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41f1fa21; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h427861b5; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h42c407d3; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42c73b8b; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_47(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4266314a; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4285f21c; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h4261b9a5; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h4241fbb4; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42467a6a; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h420a140a; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h423fdf7a; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_line_5(
  input         clock,
  input         reset,
  output        var0_ready,
  input         var0_valid,
  input  [31:0] var0_bits,
  output        var1_ready,
  input         var1_valid,
  input  [31:0] var1_bits,
  output        var2_ready,
  input         var2_valid,
  input  [31:0] var2_bits,
  output        var3_ready,
  input         var3_valid,
  input  [31:0] var3_bits,
  output        var4_ready,
  input         var4_valid,
  input  [31:0] var4_bits,
  output        var5_ready,
  input         var5_valid,
  input  [31:0] var5_bits,
  output        var6_ready,
  input         var6_valid,
  input  [31:0] var6_bits,
  output        var7_ready,
  input         var7_valid,
  input  [31:0] var7_bits,
  output        var8_ready,
  input         var8_valid,
  input  [31:0] var8_bits,
  input         var9_ready,
  output        var9_valid,
  output [31:0] var9_bits,
  input         var10_ready,
  output        var10_valid,
  output [31:0] var10_bits,
  input         var11_ready,
  output        var11_valid,
  output [31:0] var11_bits,
  input         var12_ready,
  output        var12_valid,
  output [31:0] var12_bits,
  input         var13_ready,
  output        var13_valid,
  output [31:0] var13_bits,
  input         var14_ready,
  output        var14_valid,
  output [31:0] var14_bits,
  input         var15_ready,
  output        var15_valid,
  output [31:0] var15_bits,
  input         var16_ready,
  output        var16_valid,
  output [31:0] var16_bits
);
  wire  PE_0_clock; // @[systolic_array.scala 36:26]
  wire  PE_0_reset; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_1_clock; // @[systolic_array.scala 37:26]
  wire  PE_1_reset; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_2_clock; // @[systolic_array.scala 38:26]
  wire  PE_2_reset; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_3_clock; // @[systolic_array.scala 39:26]
  wire  PE_3_reset; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_4_clock; // @[systolic_array.scala 40:26]
  wire  PE_4_reset; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_5_clock; // @[systolic_array.scala 41:26]
  wire  PE_5_reset; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_6_clock; // @[systolic_array.scala 42:26]
  wire  PE_6_reset; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_7_clock; // @[systolic_array.scala 43:26]
  wire  PE_7_reset; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_A_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_out_bits; // @[systolic_array.scala 43:26]
  PE_40 PE_0 ( // @[systolic_array.scala 36:26]
    .clock(PE_0_clock),
    .reset(PE_0_reset),
    .A_in_ready(PE_0_A_in_ready),
    .A_in_valid(PE_0_A_in_valid),
    .A_in_bits(PE_0_A_in_bits),
    .C_in_ready(PE_0_C_in_ready),
    .C_in_valid(PE_0_C_in_valid),
    .C_in_bits(PE_0_C_in_bits),
    .A_out_ready(PE_0_A_out_ready),
    .A_out_valid(PE_0_A_out_valid),
    .A_out_bits(PE_0_A_out_bits),
    .C_out_ready(PE_0_C_out_ready),
    .C_out_valid(PE_0_C_out_valid),
    .C_out_bits(PE_0_C_out_bits)
  );
  PE_41 PE_1 ( // @[systolic_array.scala 37:26]
    .clock(PE_1_clock),
    .reset(PE_1_reset),
    .A_in_ready(PE_1_A_in_ready),
    .A_in_valid(PE_1_A_in_valid),
    .A_in_bits(PE_1_A_in_bits),
    .C_in_ready(PE_1_C_in_ready),
    .C_in_valid(PE_1_C_in_valid),
    .C_in_bits(PE_1_C_in_bits),
    .A_out_ready(PE_1_A_out_ready),
    .A_out_valid(PE_1_A_out_valid),
    .A_out_bits(PE_1_A_out_bits),
    .C_out_ready(PE_1_C_out_ready),
    .C_out_valid(PE_1_C_out_valid),
    .C_out_bits(PE_1_C_out_bits)
  );
  PE_42 PE_2 ( // @[systolic_array.scala 38:26]
    .clock(PE_2_clock),
    .reset(PE_2_reset),
    .A_in_ready(PE_2_A_in_ready),
    .A_in_valid(PE_2_A_in_valid),
    .A_in_bits(PE_2_A_in_bits),
    .C_in_ready(PE_2_C_in_ready),
    .C_in_valid(PE_2_C_in_valid),
    .C_in_bits(PE_2_C_in_bits),
    .A_out_ready(PE_2_A_out_ready),
    .A_out_valid(PE_2_A_out_valid),
    .A_out_bits(PE_2_A_out_bits),
    .C_out_ready(PE_2_C_out_ready),
    .C_out_valid(PE_2_C_out_valid),
    .C_out_bits(PE_2_C_out_bits)
  );
  PE_43 PE_3 ( // @[systolic_array.scala 39:26]
    .clock(PE_3_clock),
    .reset(PE_3_reset),
    .A_in_ready(PE_3_A_in_ready),
    .A_in_valid(PE_3_A_in_valid),
    .A_in_bits(PE_3_A_in_bits),
    .C_in_ready(PE_3_C_in_ready),
    .C_in_valid(PE_3_C_in_valid),
    .C_in_bits(PE_3_C_in_bits),
    .A_out_ready(PE_3_A_out_ready),
    .A_out_valid(PE_3_A_out_valid),
    .A_out_bits(PE_3_A_out_bits),
    .C_out_ready(PE_3_C_out_ready),
    .C_out_valid(PE_3_C_out_valid),
    .C_out_bits(PE_3_C_out_bits)
  );
  PE_44 PE_4 ( // @[systolic_array.scala 40:26]
    .clock(PE_4_clock),
    .reset(PE_4_reset),
    .A_in_ready(PE_4_A_in_ready),
    .A_in_valid(PE_4_A_in_valid),
    .A_in_bits(PE_4_A_in_bits),
    .C_in_ready(PE_4_C_in_ready),
    .C_in_valid(PE_4_C_in_valid),
    .C_in_bits(PE_4_C_in_bits),
    .A_out_ready(PE_4_A_out_ready),
    .A_out_valid(PE_4_A_out_valid),
    .A_out_bits(PE_4_A_out_bits),
    .C_out_ready(PE_4_C_out_ready),
    .C_out_valid(PE_4_C_out_valid),
    .C_out_bits(PE_4_C_out_bits)
  );
  PE_45 PE_5 ( // @[systolic_array.scala 41:26]
    .clock(PE_5_clock),
    .reset(PE_5_reset),
    .A_in_ready(PE_5_A_in_ready),
    .A_in_valid(PE_5_A_in_valid),
    .A_in_bits(PE_5_A_in_bits),
    .C_in_ready(PE_5_C_in_ready),
    .C_in_valid(PE_5_C_in_valid),
    .C_in_bits(PE_5_C_in_bits),
    .A_out_ready(PE_5_A_out_ready),
    .A_out_valid(PE_5_A_out_valid),
    .A_out_bits(PE_5_A_out_bits),
    .C_out_ready(PE_5_C_out_ready),
    .C_out_valid(PE_5_C_out_valid),
    .C_out_bits(PE_5_C_out_bits)
  );
  PE_46 PE_6 ( // @[systolic_array.scala 42:26]
    .clock(PE_6_clock),
    .reset(PE_6_reset),
    .A_in_ready(PE_6_A_in_ready),
    .A_in_valid(PE_6_A_in_valid),
    .A_in_bits(PE_6_A_in_bits),
    .C_in_ready(PE_6_C_in_ready),
    .C_in_valid(PE_6_C_in_valid),
    .C_in_bits(PE_6_C_in_bits),
    .A_out_ready(PE_6_A_out_ready),
    .A_out_valid(PE_6_A_out_valid),
    .A_out_bits(PE_6_A_out_bits),
    .C_out_ready(PE_6_C_out_ready),
    .C_out_valid(PE_6_C_out_valid),
    .C_out_bits(PE_6_C_out_bits)
  );
  PE_47 PE_7 ( // @[systolic_array.scala 43:26]
    .clock(PE_7_clock),
    .reset(PE_7_reset),
    .A_in_ready(PE_7_A_in_ready),
    .A_in_valid(PE_7_A_in_valid),
    .A_in_bits(PE_7_A_in_bits),
    .C_in_ready(PE_7_C_in_ready),
    .C_in_valid(PE_7_C_in_valid),
    .C_in_bits(PE_7_C_in_bits),
    .C_out_ready(PE_7_C_out_ready),
    .C_out_valid(PE_7_C_out_valid),
    .C_out_bits(PE_7_C_out_bits)
  );
  assign var0_ready = PE_0_A_in_ready; // @[systolic_array.scala 45:19]
  assign var1_ready = PE_0_C_in_ready; // @[systolic_array.scala 46:19]
  assign var2_ready = PE_1_C_in_ready; // @[systolic_array.scala 49:19]
  assign var3_ready = PE_2_C_in_ready; // @[systolic_array.scala 52:19]
  assign var4_ready = PE_3_C_in_ready; // @[systolic_array.scala 55:19]
  assign var5_ready = PE_4_C_in_ready; // @[systolic_array.scala 58:19]
  assign var6_ready = PE_5_C_in_ready; // @[systolic_array.scala 61:19]
  assign var7_ready = PE_6_C_in_ready; // @[systolic_array.scala 64:19]
  assign var8_ready = PE_7_C_in_ready; // @[systolic_array.scala 67:19]
  assign var9_valid = PE_0_C_out_valid; // @[systolic_array.scala 47:14]
  assign var9_bits = PE_0_C_out_bits; // @[systolic_array.scala 47:14]
  assign var10_valid = PE_1_C_out_valid; // @[systolic_array.scala 50:15]
  assign var10_bits = PE_1_C_out_bits; // @[systolic_array.scala 50:15]
  assign var11_valid = PE_2_C_out_valid; // @[systolic_array.scala 53:15]
  assign var11_bits = PE_2_C_out_bits; // @[systolic_array.scala 53:15]
  assign var12_valid = PE_3_C_out_valid; // @[systolic_array.scala 56:15]
  assign var12_bits = PE_3_C_out_bits; // @[systolic_array.scala 56:15]
  assign var13_valid = PE_4_C_out_valid; // @[systolic_array.scala 59:15]
  assign var13_bits = PE_4_C_out_bits; // @[systolic_array.scala 59:15]
  assign var14_valid = PE_5_C_out_valid; // @[systolic_array.scala 62:15]
  assign var14_bits = PE_5_C_out_bits; // @[systolic_array.scala 62:15]
  assign var15_valid = PE_6_C_out_valid; // @[systolic_array.scala 65:15]
  assign var15_bits = PE_6_C_out_bits; // @[systolic_array.scala 65:15]
  assign var16_valid = PE_7_C_out_valid; // @[systolic_array.scala 68:15]
  assign var16_bits = PE_7_C_out_bits; // @[systolic_array.scala 68:15]
  assign PE_0_clock = clock;
  assign PE_0_reset = reset;
  assign PE_0_A_in_valid = var0_valid; // @[systolic_array.scala 45:19]
  assign PE_0_A_in_bits = var0_bits; // @[systolic_array.scala 45:19]
  assign PE_0_C_in_valid = var1_valid; // @[systolic_array.scala 46:19]
  assign PE_0_C_in_bits = var1_bits; // @[systolic_array.scala 46:19]
  assign PE_0_A_out_ready = PE_1_A_in_ready; // @[systolic_array.scala 48:19]
  assign PE_0_C_out_ready = var9_ready; // @[systolic_array.scala 47:14]
  assign PE_1_clock = clock;
  assign PE_1_reset = reset;
  assign PE_1_A_in_valid = PE_0_A_out_valid; // @[systolic_array.scala 48:19]
  assign PE_1_A_in_bits = PE_0_A_out_bits; // @[systolic_array.scala 48:19]
  assign PE_1_C_in_valid = var2_valid; // @[systolic_array.scala 49:19]
  assign PE_1_C_in_bits = var2_bits; // @[systolic_array.scala 49:19]
  assign PE_1_A_out_ready = PE_2_A_in_ready; // @[systolic_array.scala 51:19]
  assign PE_1_C_out_ready = var10_ready; // @[systolic_array.scala 50:15]
  assign PE_2_clock = clock;
  assign PE_2_reset = reset;
  assign PE_2_A_in_valid = PE_1_A_out_valid; // @[systolic_array.scala 51:19]
  assign PE_2_A_in_bits = PE_1_A_out_bits; // @[systolic_array.scala 51:19]
  assign PE_2_C_in_valid = var3_valid; // @[systolic_array.scala 52:19]
  assign PE_2_C_in_bits = var3_bits; // @[systolic_array.scala 52:19]
  assign PE_2_A_out_ready = PE_3_A_in_ready; // @[systolic_array.scala 54:19]
  assign PE_2_C_out_ready = var11_ready; // @[systolic_array.scala 53:15]
  assign PE_3_clock = clock;
  assign PE_3_reset = reset;
  assign PE_3_A_in_valid = PE_2_A_out_valid; // @[systolic_array.scala 54:19]
  assign PE_3_A_in_bits = PE_2_A_out_bits; // @[systolic_array.scala 54:19]
  assign PE_3_C_in_valid = var4_valid; // @[systolic_array.scala 55:19]
  assign PE_3_C_in_bits = var4_bits; // @[systolic_array.scala 55:19]
  assign PE_3_A_out_ready = PE_4_A_in_ready; // @[systolic_array.scala 57:19]
  assign PE_3_C_out_ready = var12_ready; // @[systolic_array.scala 56:15]
  assign PE_4_clock = clock;
  assign PE_4_reset = reset;
  assign PE_4_A_in_valid = PE_3_A_out_valid; // @[systolic_array.scala 57:19]
  assign PE_4_A_in_bits = PE_3_A_out_bits; // @[systolic_array.scala 57:19]
  assign PE_4_C_in_valid = var5_valid; // @[systolic_array.scala 58:19]
  assign PE_4_C_in_bits = var5_bits; // @[systolic_array.scala 58:19]
  assign PE_4_A_out_ready = PE_5_A_in_ready; // @[systolic_array.scala 60:19]
  assign PE_4_C_out_ready = var13_ready; // @[systolic_array.scala 59:15]
  assign PE_5_clock = clock;
  assign PE_5_reset = reset;
  assign PE_5_A_in_valid = PE_4_A_out_valid; // @[systolic_array.scala 60:19]
  assign PE_5_A_in_bits = PE_4_A_out_bits; // @[systolic_array.scala 60:19]
  assign PE_5_C_in_valid = var6_valid; // @[systolic_array.scala 61:19]
  assign PE_5_C_in_bits = var6_bits; // @[systolic_array.scala 61:19]
  assign PE_5_A_out_ready = PE_6_A_in_ready; // @[systolic_array.scala 63:19]
  assign PE_5_C_out_ready = var14_ready; // @[systolic_array.scala 62:15]
  assign PE_6_clock = clock;
  assign PE_6_reset = reset;
  assign PE_6_A_in_valid = PE_5_A_out_valid; // @[systolic_array.scala 63:19]
  assign PE_6_A_in_bits = PE_5_A_out_bits; // @[systolic_array.scala 63:19]
  assign PE_6_C_in_valid = var7_valid; // @[systolic_array.scala 64:19]
  assign PE_6_C_in_bits = var7_bits; // @[systolic_array.scala 64:19]
  assign PE_6_A_out_ready = PE_7_A_in_ready; // @[systolic_array.scala 66:19]
  assign PE_6_C_out_ready = var15_ready; // @[systolic_array.scala 65:15]
  assign PE_7_clock = clock;
  assign PE_7_reset = reset;
  assign PE_7_A_in_valid = PE_6_A_out_valid; // @[systolic_array.scala 66:19]
  assign PE_7_A_in_bits = PE_6_A_out_bits; // @[systolic_array.scala 66:19]
  assign PE_7_C_in_valid = var8_valid; // @[systolic_array.scala 67:19]
  assign PE_7_C_in_bits = var8_bits; // @[systolic_array.scala 67:19]
  assign PE_7_C_out_ready = var16_ready; // @[systolic_array.scala 68:15]
endmodule
module PE_48(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42641d3f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h41dae0ba; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h421f27ba; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42bdb052; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h426560db; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h4250c419; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42aa628a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_49(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4264cb8a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h41819690; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h418d3c4c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42918783; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42938f19; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h428d14a9; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42c5b885; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_50(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h424b6cf6; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h427eb7d5; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h41ab2e7c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42b485a6; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h424b5903; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h425ce6a7; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42a89300; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_51(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4248c627; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h425a125c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42980d6e; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h4192e330; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h428e1361; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h4211924f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h4292519c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_52(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42490f88; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h421d2b2b; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42219017; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h429af874; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h416ccc6f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h425a9482; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h423d576b; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_53(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h425131d5; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42010b14; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42b77741; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42b946cb; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h428a18cd; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h423e32ed; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42b32830; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_54(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h424f950c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h422b9f89; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h425abb16; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42c320b6; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h420d8c1f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h419f0d1a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h411d6b2c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_55(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h424ff0c7; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4293bf46; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42aa46d0; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42c0f048; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42822dc8; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h42b964a7; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h41dc7813; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_line_6(
  input         clock,
  input         reset,
  output        var0_ready,
  input         var0_valid,
  input  [31:0] var0_bits,
  output        var1_ready,
  input         var1_valid,
  input  [31:0] var1_bits,
  output        var2_ready,
  input         var2_valid,
  input  [31:0] var2_bits,
  output        var3_ready,
  input         var3_valid,
  input  [31:0] var3_bits,
  output        var4_ready,
  input         var4_valid,
  input  [31:0] var4_bits,
  output        var5_ready,
  input         var5_valid,
  input  [31:0] var5_bits,
  output        var6_ready,
  input         var6_valid,
  input  [31:0] var6_bits,
  output        var7_ready,
  input         var7_valid,
  input  [31:0] var7_bits,
  output        var8_ready,
  input         var8_valid,
  input  [31:0] var8_bits,
  input         var9_ready,
  output        var9_valid,
  output [31:0] var9_bits,
  input         var10_ready,
  output        var10_valid,
  output [31:0] var10_bits,
  input         var11_ready,
  output        var11_valid,
  output [31:0] var11_bits,
  input         var12_ready,
  output        var12_valid,
  output [31:0] var12_bits,
  input         var13_ready,
  output        var13_valid,
  output [31:0] var13_bits,
  input         var14_ready,
  output        var14_valid,
  output [31:0] var14_bits,
  input         var15_ready,
  output        var15_valid,
  output [31:0] var15_bits,
  input         var16_ready,
  output        var16_valid,
  output [31:0] var16_bits
);
  wire  PE_0_clock; // @[systolic_array.scala 36:26]
  wire  PE_0_reset; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_1_clock; // @[systolic_array.scala 37:26]
  wire  PE_1_reset; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_2_clock; // @[systolic_array.scala 38:26]
  wire  PE_2_reset; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_3_clock; // @[systolic_array.scala 39:26]
  wire  PE_3_reset; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_4_clock; // @[systolic_array.scala 40:26]
  wire  PE_4_reset; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_5_clock; // @[systolic_array.scala 41:26]
  wire  PE_5_reset; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_6_clock; // @[systolic_array.scala 42:26]
  wire  PE_6_reset; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_7_clock; // @[systolic_array.scala 43:26]
  wire  PE_7_reset; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_A_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_out_bits; // @[systolic_array.scala 43:26]
  PE_48 PE_0 ( // @[systolic_array.scala 36:26]
    .clock(PE_0_clock),
    .reset(PE_0_reset),
    .A_in_ready(PE_0_A_in_ready),
    .A_in_valid(PE_0_A_in_valid),
    .A_in_bits(PE_0_A_in_bits),
    .C_in_ready(PE_0_C_in_ready),
    .C_in_valid(PE_0_C_in_valid),
    .C_in_bits(PE_0_C_in_bits),
    .A_out_ready(PE_0_A_out_ready),
    .A_out_valid(PE_0_A_out_valid),
    .A_out_bits(PE_0_A_out_bits),
    .C_out_ready(PE_0_C_out_ready),
    .C_out_valid(PE_0_C_out_valid),
    .C_out_bits(PE_0_C_out_bits)
  );
  PE_49 PE_1 ( // @[systolic_array.scala 37:26]
    .clock(PE_1_clock),
    .reset(PE_1_reset),
    .A_in_ready(PE_1_A_in_ready),
    .A_in_valid(PE_1_A_in_valid),
    .A_in_bits(PE_1_A_in_bits),
    .C_in_ready(PE_1_C_in_ready),
    .C_in_valid(PE_1_C_in_valid),
    .C_in_bits(PE_1_C_in_bits),
    .A_out_ready(PE_1_A_out_ready),
    .A_out_valid(PE_1_A_out_valid),
    .A_out_bits(PE_1_A_out_bits),
    .C_out_ready(PE_1_C_out_ready),
    .C_out_valid(PE_1_C_out_valid),
    .C_out_bits(PE_1_C_out_bits)
  );
  PE_50 PE_2 ( // @[systolic_array.scala 38:26]
    .clock(PE_2_clock),
    .reset(PE_2_reset),
    .A_in_ready(PE_2_A_in_ready),
    .A_in_valid(PE_2_A_in_valid),
    .A_in_bits(PE_2_A_in_bits),
    .C_in_ready(PE_2_C_in_ready),
    .C_in_valid(PE_2_C_in_valid),
    .C_in_bits(PE_2_C_in_bits),
    .A_out_ready(PE_2_A_out_ready),
    .A_out_valid(PE_2_A_out_valid),
    .A_out_bits(PE_2_A_out_bits),
    .C_out_ready(PE_2_C_out_ready),
    .C_out_valid(PE_2_C_out_valid),
    .C_out_bits(PE_2_C_out_bits)
  );
  PE_51 PE_3 ( // @[systolic_array.scala 39:26]
    .clock(PE_3_clock),
    .reset(PE_3_reset),
    .A_in_ready(PE_3_A_in_ready),
    .A_in_valid(PE_3_A_in_valid),
    .A_in_bits(PE_3_A_in_bits),
    .C_in_ready(PE_3_C_in_ready),
    .C_in_valid(PE_3_C_in_valid),
    .C_in_bits(PE_3_C_in_bits),
    .A_out_ready(PE_3_A_out_ready),
    .A_out_valid(PE_3_A_out_valid),
    .A_out_bits(PE_3_A_out_bits),
    .C_out_ready(PE_3_C_out_ready),
    .C_out_valid(PE_3_C_out_valid),
    .C_out_bits(PE_3_C_out_bits)
  );
  PE_52 PE_4 ( // @[systolic_array.scala 40:26]
    .clock(PE_4_clock),
    .reset(PE_4_reset),
    .A_in_ready(PE_4_A_in_ready),
    .A_in_valid(PE_4_A_in_valid),
    .A_in_bits(PE_4_A_in_bits),
    .C_in_ready(PE_4_C_in_ready),
    .C_in_valid(PE_4_C_in_valid),
    .C_in_bits(PE_4_C_in_bits),
    .A_out_ready(PE_4_A_out_ready),
    .A_out_valid(PE_4_A_out_valid),
    .A_out_bits(PE_4_A_out_bits),
    .C_out_ready(PE_4_C_out_ready),
    .C_out_valid(PE_4_C_out_valid),
    .C_out_bits(PE_4_C_out_bits)
  );
  PE_53 PE_5 ( // @[systolic_array.scala 41:26]
    .clock(PE_5_clock),
    .reset(PE_5_reset),
    .A_in_ready(PE_5_A_in_ready),
    .A_in_valid(PE_5_A_in_valid),
    .A_in_bits(PE_5_A_in_bits),
    .C_in_ready(PE_5_C_in_ready),
    .C_in_valid(PE_5_C_in_valid),
    .C_in_bits(PE_5_C_in_bits),
    .A_out_ready(PE_5_A_out_ready),
    .A_out_valid(PE_5_A_out_valid),
    .A_out_bits(PE_5_A_out_bits),
    .C_out_ready(PE_5_C_out_ready),
    .C_out_valid(PE_5_C_out_valid),
    .C_out_bits(PE_5_C_out_bits)
  );
  PE_54 PE_6 ( // @[systolic_array.scala 42:26]
    .clock(PE_6_clock),
    .reset(PE_6_reset),
    .A_in_ready(PE_6_A_in_ready),
    .A_in_valid(PE_6_A_in_valid),
    .A_in_bits(PE_6_A_in_bits),
    .C_in_ready(PE_6_C_in_ready),
    .C_in_valid(PE_6_C_in_valid),
    .C_in_bits(PE_6_C_in_bits),
    .A_out_ready(PE_6_A_out_ready),
    .A_out_valid(PE_6_A_out_valid),
    .A_out_bits(PE_6_A_out_bits),
    .C_out_ready(PE_6_C_out_ready),
    .C_out_valid(PE_6_C_out_valid),
    .C_out_bits(PE_6_C_out_bits)
  );
  PE_55 PE_7 ( // @[systolic_array.scala 43:26]
    .clock(PE_7_clock),
    .reset(PE_7_reset),
    .A_in_ready(PE_7_A_in_ready),
    .A_in_valid(PE_7_A_in_valid),
    .A_in_bits(PE_7_A_in_bits),
    .C_in_ready(PE_7_C_in_ready),
    .C_in_valid(PE_7_C_in_valid),
    .C_in_bits(PE_7_C_in_bits),
    .C_out_ready(PE_7_C_out_ready),
    .C_out_valid(PE_7_C_out_valid),
    .C_out_bits(PE_7_C_out_bits)
  );
  assign var0_ready = PE_0_A_in_ready; // @[systolic_array.scala 45:19]
  assign var1_ready = PE_0_C_in_ready; // @[systolic_array.scala 46:19]
  assign var2_ready = PE_1_C_in_ready; // @[systolic_array.scala 49:19]
  assign var3_ready = PE_2_C_in_ready; // @[systolic_array.scala 52:19]
  assign var4_ready = PE_3_C_in_ready; // @[systolic_array.scala 55:19]
  assign var5_ready = PE_4_C_in_ready; // @[systolic_array.scala 58:19]
  assign var6_ready = PE_5_C_in_ready; // @[systolic_array.scala 61:19]
  assign var7_ready = PE_6_C_in_ready; // @[systolic_array.scala 64:19]
  assign var8_ready = PE_7_C_in_ready; // @[systolic_array.scala 67:19]
  assign var9_valid = PE_0_C_out_valid; // @[systolic_array.scala 47:14]
  assign var9_bits = PE_0_C_out_bits; // @[systolic_array.scala 47:14]
  assign var10_valid = PE_1_C_out_valid; // @[systolic_array.scala 50:15]
  assign var10_bits = PE_1_C_out_bits; // @[systolic_array.scala 50:15]
  assign var11_valid = PE_2_C_out_valid; // @[systolic_array.scala 53:15]
  assign var11_bits = PE_2_C_out_bits; // @[systolic_array.scala 53:15]
  assign var12_valid = PE_3_C_out_valid; // @[systolic_array.scala 56:15]
  assign var12_bits = PE_3_C_out_bits; // @[systolic_array.scala 56:15]
  assign var13_valid = PE_4_C_out_valid; // @[systolic_array.scala 59:15]
  assign var13_bits = PE_4_C_out_bits; // @[systolic_array.scala 59:15]
  assign var14_valid = PE_5_C_out_valid; // @[systolic_array.scala 62:15]
  assign var14_bits = PE_5_C_out_bits; // @[systolic_array.scala 62:15]
  assign var15_valid = PE_6_C_out_valid; // @[systolic_array.scala 65:15]
  assign var15_bits = PE_6_C_out_bits; // @[systolic_array.scala 65:15]
  assign var16_valid = PE_7_C_out_valid; // @[systolic_array.scala 68:15]
  assign var16_bits = PE_7_C_out_bits; // @[systolic_array.scala 68:15]
  assign PE_0_clock = clock;
  assign PE_0_reset = reset;
  assign PE_0_A_in_valid = var0_valid; // @[systolic_array.scala 45:19]
  assign PE_0_A_in_bits = var0_bits; // @[systolic_array.scala 45:19]
  assign PE_0_C_in_valid = var1_valid; // @[systolic_array.scala 46:19]
  assign PE_0_C_in_bits = var1_bits; // @[systolic_array.scala 46:19]
  assign PE_0_A_out_ready = PE_1_A_in_ready; // @[systolic_array.scala 48:19]
  assign PE_0_C_out_ready = var9_ready; // @[systolic_array.scala 47:14]
  assign PE_1_clock = clock;
  assign PE_1_reset = reset;
  assign PE_1_A_in_valid = PE_0_A_out_valid; // @[systolic_array.scala 48:19]
  assign PE_1_A_in_bits = PE_0_A_out_bits; // @[systolic_array.scala 48:19]
  assign PE_1_C_in_valid = var2_valid; // @[systolic_array.scala 49:19]
  assign PE_1_C_in_bits = var2_bits; // @[systolic_array.scala 49:19]
  assign PE_1_A_out_ready = PE_2_A_in_ready; // @[systolic_array.scala 51:19]
  assign PE_1_C_out_ready = var10_ready; // @[systolic_array.scala 50:15]
  assign PE_2_clock = clock;
  assign PE_2_reset = reset;
  assign PE_2_A_in_valid = PE_1_A_out_valid; // @[systolic_array.scala 51:19]
  assign PE_2_A_in_bits = PE_1_A_out_bits; // @[systolic_array.scala 51:19]
  assign PE_2_C_in_valid = var3_valid; // @[systolic_array.scala 52:19]
  assign PE_2_C_in_bits = var3_bits; // @[systolic_array.scala 52:19]
  assign PE_2_A_out_ready = PE_3_A_in_ready; // @[systolic_array.scala 54:19]
  assign PE_2_C_out_ready = var11_ready; // @[systolic_array.scala 53:15]
  assign PE_3_clock = clock;
  assign PE_3_reset = reset;
  assign PE_3_A_in_valid = PE_2_A_out_valid; // @[systolic_array.scala 54:19]
  assign PE_3_A_in_bits = PE_2_A_out_bits; // @[systolic_array.scala 54:19]
  assign PE_3_C_in_valid = var4_valid; // @[systolic_array.scala 55:19]
  assign PE_3_C_in_bits = var4_bits; // @[systolic_array.scala 55:19]
  assign PE_3_A_out_ready = PE_4_A_in_ready; // @[systolic_array.scala 57:19]
  assign PE_3_C_out_ready = var12_ready; // @[systolic_array.scala 56:15]
  assign PE_4_clock = clock;
  assign PE_4_reset = reset;
  assign PE_4_A_in_valid = PE_3_A_out_valid; // @[systolic_array.scala 57:19]
  assign PE_4_A_in_bits = PE_3_A_out_bits; // @[systolic_array.scala 57:19]
  assign PE_4_C_in_valid = var5_valid; // @[systolic_array.scala 58:19]
  assign PE_4_C_in_bits = var5_bits; // @[systolic_array.scala 58:19]
  assign PE_4_A_out_ready = PE_5_A_in_ready; // @[systolic_array.scala 60:19]
  assign PE_4_C_out_ready = var13_ready; // @[systolic_array.scala 59:15]
  assign PE_5_clock = clock;
  assign PE_5_reset = reset;
  assign PE_5_A_in_valid = PE_4_A_out_valid; // @[systolic_array.scala 60:19]
  assign PE_5_A_in_bits = PE_4_A_out_bits; // @[systolic_array.scala 60:19]
  assign PE_5_C_in_valid = var6_valid; // @[systolic_array.scala 61:19]
  assign PE_5_C_in_bits = var6_bits; // @[systolic_array.scala 61:19]
  assign PE_5_A_out_ready = PE_6_A_in_ready; // @[systolic_array.scala 63:19]
  assign PE_5_C_out_ready = var14_ready; // @[systolic_array.scala 62:15]
  assign PE_6_clock = clock;
  assign PE_6_reset = reset;
  assign PE_6_A_in_valid = PE_5_A_out_valid; // @[systolic_array.scala 63:19]
  assign PE_6_A_in_bits = PE_5_A_out_bits; // @[systolic_array.scala 63:19]
  assign PE_6_C_in_valid = var7_valid; // @[systolic_array.scala 64:19]
  assign PE_6_C_in_bits = var7_bits; // @[systolic_array.scala 64:19]
  assign PE_6_A_out_ready = PE_7_A_in_ready; // @[systolic_array.scala 66:19]
  assign PE_6_C_out_ready = var15_ready; // @[systolic_array.scala 65:15]
  assign PE_7_clock = clock;
  assign PE_7_reset = reset;
  assign PE_7_A_in_valid = PE_6_A_out_valid; // @[systolic_array.scala 66:19]
  assign PE_7_A_in_bits = PE_6_A_out_bits; // @[systolic_array.scala 66:19]
  assign PE_7_C_in_valid = var8_valid; // @[systolic_array.scala 67:19]
  assign PE_7_C_in_bits = var8_bits; // @[systolic_array.scala 67:19]
  assign PE_7_C_out_ready = var16_ready; // @[systolic_array.scala 68:15]
endmodule
module PE_56(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h424f4ba8; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h426886ba; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42b4a2ee; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h4219c219; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42b73fe3; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h3fd08b1c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h420ea69a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_57(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h424d8101; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4052bae0; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42182929; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h422fa65c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h422c7292; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h4216369a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h423c4067; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_58(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h424d2546; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4290a656; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h40f2b502; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h42340735; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h41568c83; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h428179ed; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h41eabe53; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_59(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h424dd38f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4274a798; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42aae686; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41b76b2d; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h41eec0ef; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h42a62c8a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h422c0b1e; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_60(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4255353a; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4198ccc3; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42b9abdf; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h41a3081c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h42a36f45; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h40451b79; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h41e05624; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_61(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4255ff08; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h41d9d533; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h419eb43c; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h4267e22f; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h41d88103; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h40517c39; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h40e0d4ca; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_62(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         A_out_ready,
  output        A_out_valid,
  output [31:0] A_out_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  wire  ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign A_out_bits = A_buf_valid_out[32:1]; // @[systolic.scala 56:33]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = A_out_ready & C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h42543461; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h4290c7c7; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42871eb7; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h427dc670; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h429d19a7; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h421cca05; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h419368cb; // @[systolic.scala 26:49]
    end else if (ce) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_63(
  input         clock,
  input         reset,
  output        A_in_ready,
  input         A_in_valid,
  input  [31:0] A_in_bits,
  output        C_in_ready,
  input         C_in_valid,
  input  [31:0] C_in_bits,
  input         C_out_ready,
  output        C_out_valid,
  output [31:0] C_out_bits
);
`ifdef RANDOMIZE_REG_INIT
  reg [31:0] _RAND_0;
  reg [31:0] _RAND_1;
  reg [31:0] _RAND_2;
  reg [31:0] _RAND_3;
  reg [31:0] _RAND_4;
  reg [31:0] _RAND_5;
  reg [31:0] _RAND_6;
`endif // RANDOMIZE_REG_INIT
  wire  mul_clock; // @[systolic.scala 37:19]
  wire [31:0] mul_operand0; // @[systolic.scala 37:19]
  wire [31:0] mul_operand1; // @[systolic.scala 37:19]
  wire  mul_ce; // @[systolic.scala 37:19]
  wire [31:0] mul_result; // @[systolic.scala 37:19]
  wire  add_clock; // @[systolic.scala 40:19]
  wire [31:0] add_operand0; // @[systolic.scala 40:19]
  wire [31:0] add_operand1; // @[systolic.scala 40:19]
  wire  add_ce; // @[systolic.scala 40:19]
  wire [31:0] add_result; // @[systolic.scala 40:19]
  wire  A_buf_clock; // @[systolic.scala 43:21]
  wire  A_buf_reset; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_in; // @[systolic.scala 43:21]
  wire  A_buf_ready_in; // @[systolic.scala 43:21]
  wire [32:0] A_buf_valid_out; // @[systolic.scala 43:21]
  wire  C_buf_clock; // @[systolic.scala 50:21]
  wire  C_buf_reset; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_in; // @[systolic.scala 50:21]
  wire  C_buf_ready_in; // @[systolic.scala 50:21]
  wire [31:0] C_buf_valid_out; // @[systolic.scala 50:21]
  wire  valid_in = A_in_valid & C_in_valid; // @[systolic.scala 20:29]
  reg [31:0] B; // @[systolic.scala 26:49]
  reg [31:0] B_reg_1; // @[systolic.scala 26:49]
  reg [31:0] B_reg_2; // @[systolic.scala 26:49]
  reg [31:0] B_reg_3; // @[systolic.scala 26:49]
  reg [31:0] B_reg_4; // @[systolic.scala 26:49]
  reg [31:0] B_reg_5; // @[systolic.scala 26:49]
  reg [31:0] B_reg_6; // @[systolic.scala 26:49]
  wire [32:0] _A_buf_valid_in_T = {A_in_bits, 1'h0}; // @[systolic.scala 44:32]
  wire [32:0] _GEN_7 = {{32'd0}, valid_in}; // @[systolic.scala 44:38]
  wire [32:0] _valid_out_T = A_buf_valid_out & 33'h1; // @[systolic.scala 59:32]
  MulFBase mul ( // @[systolic.scala 37:19]
    .clock(mul_clock),
    .operand0(mul_operand0),
    .operand1(mul_operand1),
    .ce(mul_ce),
    .result(mul_result)
  );
  AddSubFBase add ( // @[systolic.scala 40:19]
    .clock(add_clock),
    .operand0(add_operand0),
    .operand1(add_operand1),
    .ce(add_ce),
    .result(add_result)
  );
  DelayBuffer A_buf ( // @[systolic.scala 43:21]
    .clock(A_buf_clock),
    .reset(A_buf_reset),
    .valid_in(A_buf_valid_in),
    .ready_in(A_buf_ready_in),
    .valid_out(A_buf_valid_out)
  );
  DelayBuffer_1 C_buf ( // @[systolic.scala 50:21]
    .clock(C_buf_clock),
    .reset(C_buf_reset),
    .valid_in(C_buf_valid_in),
    .ready_in(C_buf_ready_in),
    .valid_out(C_buf_valid_out)
  );
  assign A_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_in_ready = C_out_ready; // @[systolic.scala 22:31]
  assign C_out_valid = _valid_out_T[0]; // @[systolic.scala 21:23 systolic.scala 59:13]
  assign C_out_bits = add_result; // @[systolic.scala 57:14]
  assign mul_clock = clock;
  assign mul_operand0 = A_in_bits; // @[systolic.scala 47:16]
  assign mul_operand1 = B; // @[systolic.scala 48:16]
  assign mul_ce = C_out_ready; // @[systolic.scala 22:31]
  assign add_clock = clock;
  assign add_operand0 = C_buf_valid_out; // @[systolic.scala 53:16]
  assign add_operand1 = mul_result; // @[systolic.scala 54:16]
  assign add_ce = C_out_ready; // @[systolic.scala 22:31]
  assign A_buf_clock = clock;
  assign A_buf_reset = reset;
  assign A_buf_valid_in = _A_buf_valid_in_T | _GEN_7; // @[systolic.scala 44:38]
  assign A_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  assign C_buf_clock = clock;
  assign C_buf_reset = reset;
  assign C_buf_valid_in = C_in_bits; // @[systolic.scala 51:18]
  assign C_buf_ready_in = C_out_ready; // @[systolic.scala 22:31]
  always @(posedge clock) begin
    if (reset) begin // @[systolic.scala 26:49]
      B <= 32'h4254fe2f; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B <= B_reg_1; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_1 <= 32'h42a109e3; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_1 <= B_reg_2; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_2 <= 32'h42bd1fe8; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_2 <= B_reg_3; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_3 <= 32'h3f849216; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_3 <= B_reg_4; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_4 <= 32'h41bf2a8d; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_4 <= B_reg_5; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_5 <= 32'h421d9011; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_5 <= B_reg_6; // @[systolic.scala 31:20]
    end
    if (reset) begin // @[systolic.scala 26:49]
      B_reg_6 <= 32'h42c2d1f6; // @[systolic.scala 26:49]
    end else if (C_out_ready) begin // @[systolic.scala 29:12]
      B_reg_6 <= B; // @[systolic.scala 33:14]
    end
  end
// Register and memory initialization
`ifdef RANDOMIZE_GARBAGE_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_INVALID_ASSIGN
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_REG_INIT
`define RANDOMIZE
`endif
`ifdef RANDOMIZE_MEM_INIT
`define RANDOMIZE
`endif
`ifndef RANDOM
`define RANDOM $random
`endif
`ifdef RANDOMIZE_MEM_INIT
  integer initvar;
`endif
`ifndef SYNTHESIS
`ifdef FIRRTL_BEFORE_INITIAL
`FIRRTL_BEFORE_INITIAL
`endif
initial begin
  `ifdef RANDOMIZE
    `ifdef INIT_RANDOM
      `INIT_RANDOM
    `endif
    `ifndef VERILATOR
      `ifdef RANDOMIZE_DELAY
        #`RANDOMIZE_DELAY begin end
      `else
        #0.002 begin end
      `endif
    `endif
`ifdef RANDOMIZE_REG_INIT
  _RAND_0 = {1{`RANDOM}};
  B = _RAND_0[31:0];
  _RAND_1 = {1{`RANDOM}};
  B_reg_1 = _RAND_1[31:0];
  _RAND_2 = {1{`RANDOM}};
  B_reg_2 = _RAND_2[31:0];
  _RAND_3 = {1{`RANDOM}};
  B_reg_3 = _RAND_3[31:0];
  _RAND_4 = {1{`RANDOM}};
  B_reg_4 = _RAND_4[31:0];
  _RAND_5 = {1{`RANDOM}};
  B_reg_5 = _RAND_5[31:0];
  _RAND_6 = {1{`RANDOM}};
  B_reg_6 = _RAND_6[31:0];
`endif // RANDOMIZE_REG_INIT
  `endif // RANDOMIZE
end // initial
`ifdef FIRRTL_AFTER_INITIAL
`FIRRTL_AFTER_INITIAL
`endif
`endif // SYNTHESIS
endmodule
module PE_line_7(
  input         clock,
  input         reset,
  output        var0_ready,
  input         var0_valid,
  input  [31:0] var0_bits,
  output        var1_ready,
  input         var1_valid,
  input  [31:0] var1_bits,
  output        var2_ready,
  input         var2_valid,
  input  [31:0] var2_bits,
  output        var3_ready,
  input         var3_valid,
  input  [31:0] var3_bits,
  output        var4_ready,
  input         var4_valid,
  input  [31:0] var4_bits,
  output        var5_ready,
  input         var5_valid,
  input  [31:0] var5_bits,
  output        var6_ready,
  input         var6_valid,
  input  [31:0] var6_bits,
  output        var7_ready,
  input         var7_valid,
  input  [31:0] var7_bits,
  output        var8_ready,
  input         var8_valid,
  input  [31:0] var8_bits,
  input         var9_ready,
  output        var9_valid,
  output [31:0] var9_bits,
  input         var10_ready,
  output        var10_valid,
  output [31:0] var10_bits,
  input         var11_ready,
  output        var11_valid,
  output [31:0] var11_bits,
  input         var12_ready,
  output        var12_valid,
  output [31:0] var12_bits,
  input         var13_ready,
  output        var13_valid,
  output [31:0] var13_bits,
  input         var14_ready,
  output        var14_valid,
  output [31:0] var14_bits,
  input         var15_ready,
  output        var15_valid,
  output [31:0] var15_bits,
  input         var16_ready,
  output        var16_valid,
  output [31:0] var16_bits
);
  wire  PE_0_clock; // @[systolic_array.scala 36:26]
  wire  PE_0_reset; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_in_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_in_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_A_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_A_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_ready; // @[systolic_array.scala 36:26]
  wire  PE_0_C_out_valid; // @[systolic_array.scala 36:26]
  wire [31:0] PE_0_C_out_bits; // @[systolic_array.scala 36:26]
  wire  PE_1_clock; // @[systolic_array.scala 37:26]
  wire  PE_1_reset; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_in_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_in_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_A_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_A_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_ready; // @[systolic_array.scala 37:26]
  wire  PE_1_C_out_valid; // @[systolic_array.scala 37:26]
  wire [31:0] PE_1_C_out_bits; // @[systolic_array.scala 37:26]
  wire  PE_2_clock; // @[systolic_array.scala 38:26]
  wire  PE_2_reset; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_in_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_in_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_A_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_A_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_ready; // @[systolic_array.scala 38:26]
  wire  PE_2_C_out_valid; // @[systolic_array.scala 38:26]
  wire [31:0] PE_2_C_out_bits; // @[systolic_array.scala 38:26]
  wire  PE_3_clock; // @[systolic_array.scala 39:26]
  wire  PE_3_reset; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_in_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_in_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_A_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_A_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_ready; // @[systolic_array.scala 39:26]
  wire  PE_3_C_out_valid; // @[systolic_array.scala 39:26]
  wire [31:0] PE_3_C_out_bits; // @[systolic_array.scala 39:26]
  wire  PE_4_clock; // @[systolic_array.scala 40:26]
  wire  PE_4_reset; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_in_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_in_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_A_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_A_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_ready; // @[systolic_array.scala 40:26]
  wire  PE_4_C_out_valid; // @[systolic_array.scala 40:26]
  wire [31:0] PE_4_C_out_bits; // @[systolic_array.scala 40:26]
  wire  PE_5_clock; // @[systolic_array.scala 41:26]
  wire  PE_5_reset; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_in_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_in_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_A_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_A_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_ready; // @[systolic_array.scala 41:26]
  wire  PE_5_C_out_valid; // @[systolic_array.scala 41:26]
  wire [31:0] PE_5_C_out_bits; // @[systolic_array.scala 41:26]
  wire  PE_6_clock; // @[systolic_array.scala 42:26]
  wire  PE_6_reset; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_in_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_in_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_A_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_A_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_ready; // @[systolic_array.scala 42:26]
  wire  PE_6_C_out_valid; // @[systolic_array.scala 42:26]
  wire [31:0] PE_6_C_out_bits; // @[systolic_array.scala 42:26]
  wire  PE_7_clock; // @[systolic_array.scala 43:26]
  wire  PE_7_reset; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_A_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_A_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_in_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_in_bits; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_ready; // @[systolic_array.scala 43:26]
  wire  PE_7_C_out_valid; // @[systolic_array.scala 43:26]
  wire [31:0] PE_7_C_out_bits; // @[systolic_array.scala 43:26]
  PE_56 PE_0 ( // @[systolic_array.scala 36:26]
    .clock(PE_0_clock),
    .reset(PE_0_reset),
    .A_in_ready(PE_0_A_in_ready),
    .A_in_valid(PE_0_A_in_valid),
    .A_in_bits(PE_0_A_in_bits),
    .C_in_ready(PE_0_C_in_ready),
    .C_in_valid(PE_0_C_in_valid),
    .C_in_bits(PE_0_C_in_bits),
    .A_out_ready(PE_0_A_out_ready),
    .A_out_valid(PE_0_A_out_valid),
    .A_out_bits(PE_0_A_out_bits),
    .C_out_ready(PE_0_C_out_ready),
    .C_out_valid(PE_0_C_out_valid),
    .C_out_bits(PE_0_C_out_bits)
  );
  PE_57 PE_1 ( // @[systolic_array.scala 37:26]
    .clock(PE_1_clock),
    .reset(PE_1_reset),
    .A_in_ready(PE_1_A_in_ready),
    .A_in_valid(PE_1_A_in_valid),
    .A_in_bits(PE_1_A_in_bits),
    .C_in_ready(PE_1_C_in_ready),
    .C_in_valid(PE_1_C_in_valid),
    .C_in_bits(PE_1_C_in_bits),
    .A_out_ready(PE_1_A_out_ready),
    .A_out_valid(PE_1_A_out_valid),
    .A_out_bits(PE_1_A_out_bits),
    .C_out_ready(PE_1_C_out_ready),
    .C_out_valid(PE_1_C_out_valid),
    .C_out_bits(PE_1_C_out_bits)
  );
  PE_58 PE_2 ( // @[systolic_array.scala 38:26]
    .clock(PE_2_clock),
    .reset(PE_2_reset),
    .A_in_ready(PE_2_A_in_ready),
    .A_in_valid(PE_2_A_in_valid),
    .A_in_bits(PE_2_A_in_bits),
    .C_in_ready(PE_2_C_in_ready),
    .C_in_valid(PE_2_C_in_valid),
    .C_in_bits(PE_2_C_in_bits),
    .A_out_ready(PE_2_A_out_ready),
    .A_out_valid(PE_2_A_out_valid),
    .A_out_bits(PE_2_A_out_bits),
    .C_out_ready(PE_2_C_out_ready),
    .C_out_valid(PE_2_C_out_valid),
    .C_out_bits(PE_2_C_out_bits)
  );
  PE_59 PE_3 ( // @[systolic_array.scala 39:26]
    .clock(PE_3_clock),
    .reset(PE_3_reset),
    .A_in_ready(PE_3_A_in_ready),
    .A_in_valid(PE_3_A_in_valid),
    .A_in_bits(PE_3_A_in_bits),
    .C_in_ready(PE_3_C_in_ready),
    .C_in_valid(PE_3_C_in_valid),
    .C_in_bits(PE_3_C_in_bits),
    .A_out_ready(PE_3_A_out_ready),
    .A_out_valid(PE_3_A_out_valid),
    .A_out_bits(PE_3_A_out_bits),
    .C_out_ready(PE_3_C_out_ready),
    .C_out_valid(PE_3_C_out_valid),
    .C_out_bits(PE_3_C_out_bits)
  );
  PE_60 PE_4 ( // @[systolic_array.scala 40:26]
    .clock(PE_4_clock),
    .reset(PE_4_reset),
    .A_in_ready(PE_4_A_in_ready),
    .A_in_valid(PE_4_A_in_valid),
    .A_in_bits(PE_4_A_in_bits),
    .C_in_ready(PE_4_C_in_ready),
    .C_in_valid(PE_4_C_in_valid),
    .C_in_bits(PE_4_C_in_bits),
    .A_out_ready(PE_4_A_out_ready),
    .A_out_valid(PE_4_A_out_valid),
    .A_out_bits(PE_4_A_out_bits),
    .C_out_ready(PE_4_C_out_ready),
    .C_out_valid(PE_4_C_out_valid),
    .C_out_bits(PE_4_C_out_bits)
  );
  PE_61 PE_5 ( // @[systolic_array.scala 41:26]
    .clock(PE_5_clock),
    .reset(PE_5_reset),
    .A_in_ready(PE_5_A_in_ready),
    .A_in_valid(PE_5_A_in_valid),
    .A_in_bits(PE_5_A_in_bits),
    .C_in_ready(PE_5_C_in_ready),
    .C_in_valid(PE_5_C_in_valid),
    .C_in_bits(PE_5_C_in_bits),
    .A_out_ready(PE_5_A_out_ready),
    .A_out_valid(PE_5_A_out_valid),
    .A_out_bits(PE_5_A_out_bits),
    .C_out_ready(PE_5_C_out_ready),
    .C_out_valid(PE_5_C_out_valid),
    .C_out_bits(PE_5_C_out_bits)
  );
  PE_62 PE_6 ( // @[systolic_array.scala 42:26]
    .clock(PE_6_clock),
    .reset(PE_6_reset),
    .A_in_ready(PE_6_A_in_ready),
    .A_in_valid(PE_6_A_in_valid),
    .A_in_bits(PE_6_A_in_bits),
    .C_in_ready(PE_6_C_in_ready),
    .C_in_valid(PE_6_C_in_valid),
    .C_in_bits(PE_6_C_in_bits),
    .A_out_ready(PE_6_A_out_ready),
    .A_out_valid(PE_6_A_out_valid),
    .A_out_bits(PE_6_A_out_bits),
    .C_out_ready(PE_6_C_out_ready),
    .C_out_valid(PE_6_C_out_valid),
    .C_out_bits(PE_6_C_out_bits)
  );
  PE_63 PE_7 ( // @[systolic_array.scala 43:26]
    .clock(PE_7_clock),
    .reset(PE_7_reset),
    .A_in_ready(PE_7_A_in_ready),
    .A_in_valid(PE_7_A_in_valid),
    .A_in_bits(PE_7_A_in_bits),
    .C_in_ready(PE_7_C_in_ready),
    .C_in_valid(PE_7_C_in_valid),
    .C_in_bits(PE_7_C_in_bits),
    .C_out_ready(PE_7_C_out_ready),
    .C_out_valid(PE_7_C_out_valid),
    .C_out_bits(PE_7_C_out_bits)
  );
  assign var0_ready = PE_0_A_in_ready; // @[systolic_array.scala 45:19]
  assign var1_ready = PE_0_C_in_ready; // @[systolic_array.scala 46:19]
  assign var2_ready = PE_1_C_in_ready; // @[systolic_array.scala 49:19]
  assign var3_ready = PE_2_C_in_ready; // @[systolic_array.scala 52:19]
  assign var4_ready = PE_3_C_in_ready; // @[systolic_array.scala 55:19]
  assign var5_ready = PE_4_C_in_ready; // @[systolic_array.scala 58:19]
  assign var6_ready = PE_5_C_in_ready; // @[systolic_array.scala 61:19]
  assign var7_ready = PE_6_C_in_ready; // @[systolic_array.scala 64:19]
  assign var8_ready = PE_7_C_in_ready; // @[systolic_array.scala 67:19]
  assign var9_valid = PE_0_C_out_valid; // @[systolic_array.scala 47:14]
  assign var9_bits = PE_0_C_out_bits; // @[systolic_array.scala 47:14]
  assign var10_valid = PE_1_C_out_valid; // @[systolic_array.scala 50:15]
  assign var10_bits = PE_1_C_out_bits; // @[systolic_array.scala 50:15]
  assign var11_valid = PE_2_C_out_valid; // @[systolic_array.scala 53:15]
  assign var11_bits = PE_2_C_out_bits; // @[systolic_array.scala 53:15]
  assign var12_valid = PE_3_C_out_valid; // @[systolic_array.scala 56:15]
  assign var12_bits = PE_3_C_out_bits; // @[systolic_array.scala 56:15]
  assign var13_valid = PE_4_C_out_valid; // @[systolic_array.scala 59:15]
  assign var13_bits = PE_4_C_out_bits; // @[systolic_array.scala 59:15]
  assign var14_valid = PE_5_C_out_valid; // @[systolic_array.scala 62:15]
  assign var14_bits = PE_5_C_out_bits; // @[systolic_array.scala 62:15]
  assign var15_valid = PE_6_C_out_valid; // @[systolic_array.scala 65:15]
  assign var15_bits = PE_6_C_out_bits; // @[systolic_array.scala 65:15]
  assign var16_valid = PE_7_C_out_valid; // @[systolic_array.scala 68:15]
  assign var16_bits = PE_7_C_out_bits; // @[systolic_array.scala 68:15]
  assign PE_0_clock = clock;
  assign PE_0_reset = reset;
  assign PE_0_A_in_valid = var0_valid; // @[systolic_array.scala 45:19]
  assign PE_0_A_in_bits = var0_bits; // @[systolic_array.scala 45:19]
  assign PE_0_C_in_valid = var1_valid; // @[systolic_array.scala 46:19]
  assign PE_0_C_in_bits = var1_bits; // @[systolic_array.scala 46:19]
  assign PE_0_A_out_ready = PE_1_A_in_ready; // @[systolic_array.scala 48:19]
  assign PE_0_C_out_ready = var9_ready; // @[systolic_array.scala 47:14]
  assign PE_1_clock = clock;
  assign PE_1_reset = reset;
  assign PE_1_A_in_valid = PE_0_A_out_valid; // @[systolic_array.scala 48:19]
  assign PE_1_A_in_bits = PE_0_A_out_bits; // @[systolic_array.scala 48:19]
  assign PE_1_C_in_valid = var2_valid; // @[systolic_array.scala 49:19]
  assign PE_1_C_in_bits = var2_bits; // @[systolic_array.scala 49:19]
  assign PE_1_A_out_ready = PE_2_A_in_ready; // @[systolic_array.scala 51:19]
  assign PE_1_C_out_ready = var10_ready; // @[systolic_array.scala 50:15]
  assign PE_2_clock = clock;
  assign PE_2_reset = reset;
  assign PE_2_A_in_valid = PE_1_A_out_valid; // @[systolic_array.scala 51:19]
  assign PE_2_A_in_bits = PE_1_A_out_bits; // @[systolic_array.scala 51:19]
  assign PE_2_C_in_valid = var3_valid; // @[systolic_array.scala 52:19]
  assign PE_2_C_in_bits = var3_bits; // @[systolic_array.scala 52:19]
  assign PE_2_A_out_ready = PE_3_A_in_ready; // @[systolic_array.scala 54:19]
  assign PE_2_C_out_ready = var11_ready; // @[systolic_array.scala 53:15]
  assign PE_3_clock = clock;
  assign PE_3_reset = reset;
  assign PE_3_A_in_valid = PE_2_A_out_valid; // @[systolic_array.scala 54:19]
  assign PE_3_A_in_bits = PE_2_A_out_bits; // @[systolic_array.scala 54:19]
  assign PE_3_C_in_valid = var4_valid; // @[systolic_array.scala 55:19]
  assign PE_3_C_in_bits = var4_bits; // @[systolic_array.scala 55:19]
  assign PE_3_A_out_ready = PE_4_A_in_ready; // @[systolic_array.scala 57:19]
  assign PE_3_C_out_ready = var12_ready; // @[systolic_array.scala 56:15]
  assign PE_4_clock = clock;
  assign PE_4_reset = reset;
  assign PE_4_A_in_valid = PE_3_A_out_valid; // @[systolic_array.scala 57:19]
  assign PE_4_A_in_bits = PE_3_A_out_bits; // @[systolic_array.scala 57:19]
  assign PE_4_C_in_valid = var5_valid; // @[systolic_array.scala 58:19]
  assign PE_4_C_in_bits = var5_bits; // @[systolic_array.scala 58:19]
  assign PE_4_A_out_ready = PE_5_A_in_ready; // @[systolic_array.scala 60:19]
  assign PE_4_C_out_ready = var13_ready; // @[systolic_array.scala 59:15]
  assign PE_5_clock = clock;
  assign PE_5_reset = reset;
  assign PE_5_A_in_valid = PE_4_A_out_valid; // @[systolic_array.scala 60:19]
  assign PE_5_A_in_bits = PE_4_A_out_bits; // @[systolic_array.scala 60:19]
  assign PE_5_C_in_valid = var6_valid; // @[systolic_array.scala 61:19]
  assign PE_5_C_in_bits = var6_bits; // @[systolic_array.scala 61:19]
  assign PE_5_A_out_ready = PE_6_A_in_ready; // @[systolic_array.scala 63:19]
  assign PE_5_C_out_ready = var14_ready; // @[systolic_array.scala 62:15]
  assign PE_6_clock = clock;
  assign PE_6_reset = reset;
  assign PE_6_A_in_valid = PE_5_A_out_valid; // @[systolic_array.scala 63:19]
  assign PE_6_A_in_bits = PE_5_A_out_bits; // @[systolic_array.scala 63:19]
  assign PE_6_C_in_valid = var7_valid; // @[systolic_array.scala 64:19]
  assign PE_6_C_in_bits = var7_bits; // @[systolic_array.scala 64:19]
  assign PE_6_A_out_ready = PE_7_A_in_ready; // @[systolic_array.scala 66:19]
  assign PE_6_C_out_ready = var15_ready; // @[systolic_array.scala 65:15]
  assign PE_7_clock = clock;
  assign PE_7_reset = reset;
  assign PE_7_A_in_valid = PE_6_A_out_valid; // @[systolic_array.scala 66:19]
  assign PE_7_A_in_bits = PE_6_A_out_bits; // @[systolic_array.scala 66:19]
  assign PE_7_C_in_valid = var8_valid; // @[systolic_array.scala 67:19]
  assign PE_7_C_in_bits = var8_bits; // @[systolic_array.scala 67:19]
  assign PE_7_C_out_ready = var16_ready; // @[systolic_array.scala 68:15]
endmodule
module main_hec(
  input         clock,
  input         reset,
  output        var17_ready,
  input         var17_valid,
  input  [31:0] var17_bits,
  output        var18_ready,
  input         var18_valid,
  input  [31:0] var18_bits,
  output        var19_ready,
  input         var19_valid,
  input  [31:0] var19_bits,
  output        var20_ready,
  input         var20_valid,
  input  [31:0] var20_bits,
  output        var21_ready,
  input         var21_valid,
  input  [31:0] var21_bits,
  output        var22_ready,
  input         var22_valid,
  input  [31:0] var22_bits,
  output        var23_ready,
  input         var23_valid,
  input  [31:0] var23_bits,
  output        var24_ready,
  input         var24_valid,
  input  [31:0] var24_bits,
  output        var25_ready,
  input         var25_valid,
  input  [31:0] var25_bits,
  output        var26_ready,
  input         var26_valid,
  input  [31:0] var26_bits,
  output        var27_ready,
  input         var27_valid,
  input  [31:0] var27_bits,
  output        var28_ready,
  input         var28_valid,
  input  [31:0] var28_bits,
  output        var29_ready,
  input         var29_valid,
  input  [31:0] var29_bits,
  output        var30_ready,
  input         var30_valid,
  input  [31:0] var30_bits,
  output        var31_ready,
  input         var31_valid,
  input  [31:0] var31_bits,
  output        var32_ready,
  input         var32_valid,
  input  [31:0] var32_bits,
  input         var33_ready,
  output        var33_valid,
  output [31:0] var33_bits,
  input         var34_ready,
  output        var34_valid,
  output [31:0] var34_bits,
  input         var35_ready,
  output        var35_valid,
  output [31:0] var35_bits,
  input         var36_ready,
  output        var36_valid,
  output [31:0] var36_bits,
  input         var37_ready,
  output        var37_valid,
  output [31:0] var37_bits,
  input         var38_ready,
  output        var38_valid,
  output [31:0] var38_bits,
  input         var39_ready,
  output        var39_valid,
  output [31:0] var39_bits,
  input         var40_ready,
  output        var40_valid,
  output [31:0] var40_bits
);
  wire  Line_0_clock; // @[systolic_array.scala 103:28]
  wire  Line_0_reset; // @[systolic_array.scala 103:28]
  wire  Line_0_var0_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var0_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var0_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var1_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var1_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var1_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var2_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var2_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var2_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var3_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var3_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var3_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var4_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var4_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var4_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var5_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var5_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var5_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var6_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var6_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var6_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var7_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var7_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var7_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var8_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var8_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var8_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var9_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var9_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var9_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var10_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var10_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var10_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var11_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var11_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var11_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var12_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var12_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var12_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var13_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var13_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var13_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var14_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var14_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var14_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var15_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var15_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var15_bits; // @[systolic_array.scala 103:28]
  wire  Line_0_var16_ready; // @[systolic_array.scala 103:28]
  wire  Line_0_var16_valid; // @[systolic_array.scala 103:28]
  wire [31:0] Line_0_var16_bits; // @[systolic_array.scala 103:28]
  wire  Line_1_clock; // @[systolic_array.scala 104:28]
  wire  Line_1_reset; // @[systolic_array.scala 104:28]
  wire  Line_1_var0_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var0_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var0_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var1_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var1_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var1_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var2_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var2_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var2_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var3_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var3_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var3_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var4_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var4_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var4_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var5_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var5_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var5_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var6_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var6_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var6_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var7_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var7_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var7_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var8_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var8_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var8_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var9_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var9_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var9_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var10_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var10_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var10_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var11_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var11_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var11_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var12_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var12_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var12_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var13_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var13_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var13_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var14_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var14_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var14_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var15_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var15_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var15_bits; // @[systolic_array.scala 104:28]
  wire  Line_1_var16_ready; // @[systolic_array.scala 104:28]
  wire  Line_1_var16_valid; // @[systolic_array.scala 104:28]
  wire [31:0] Line_1_var16_bits; // @[systolic_array.scala 104:28]
  wire  Line_2_clock; // @[systolic_array.scala 105:28]
  wire  Line_2_reset; // @[systolic_array.scala 105:28]
  wire  Line_2_var0_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var0_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var0_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var1_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var1_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var1_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var2_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var2_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var2_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var3_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var3_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var3_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var4_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var4_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var4_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var5_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var5_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var5_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var6_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var6_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var6_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var7_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var7_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var7_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var8_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var8_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var8_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var9_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var9_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var9_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var10_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var10_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var10_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var11_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var11_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var11_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var12_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var12_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var12_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var13_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var13_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var13_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var14_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var14_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var14_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var15_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var15_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var15_bits; // @[systolic_array.scala 105:28]
  wire  Line_2_var16_ready; // @[systolic_array.scala 105:28]
  wire  Line_2_var16_valid; // @[systolic_array.scala 105:28]
  wire [31:0] Line_2_var16_bits; // @[systolic_array.scala 105:28]
  wire  Line_3_clock; // @[systolic_array.scala 106:28]
  wire  Line_3_reset; // @[systolic_array.scala 106:28]
  wire  Line_3_var0_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var0_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var0_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var1_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var1_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var1_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var2_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var2_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var2_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var3_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var3_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var3_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var4_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var4_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var4_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var5_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var5_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var5_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var6_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var6_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var6_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var7_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var7_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var7_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var8_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var8_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var8_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var9_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var9_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var9_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var10_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var10_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var10_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var11_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var11_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var11_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var12_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var12_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var12_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var13_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var13_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var13_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var14_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var14_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var14_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var15_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var15_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var15_bits; // @[systolic_array.scala 106:28]
  wire  Line_3_var16_ready; // @[systolic_array.scala 106:28]
  wire  Line_3_var16_valid; // @[systolic_array.scala 106:28]
  wire [31:0] Line_3_var16_bits; // @[systolic_array.scala 106:28]
  wire  Line_4_clock; // @[systolic_array.scala 107:28]
  wire  Line_4_reset; // @[systolic_array.scala 107:28]
  wire  Line_4_var0_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var0_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var0_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var1_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var1_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var1_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var2_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var2_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var2_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var3_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var3_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var3_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var4_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var4_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var4_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var5_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var5_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var5_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var6_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var6_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var6_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var7_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var7_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var7_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var8_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var8_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var8_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var9_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var9_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var9_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var10_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var10_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var10_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var11_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var11_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var11_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var12_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var12_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var12_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var13_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var13_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var13_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var14_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var14_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var14_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var15_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var15_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var15_bits; // @[systolic_array.scala 107:28]
  wire  Line_4_var16_ready; // @[systolic_array.scala 107:28]
  wire  Line_4_var16_valid; // @[systolic_array.scala 107:28]
  wire [31:0] Line_4_var16_bits; // @[systolic_array.scala 107:28]
  wire  Line_5_clock; // @[systolic_array.scala 108:28]
  wire  Line_5_reset; // @[systolic_array.scala 108:28]
  wire  Line_5_var0_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var0_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var0_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var1_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var1_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var1_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var2_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var2_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var2_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var3_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var3_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var3_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var4_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var4_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var4_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var5_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var5_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var5_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var6_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var6_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var6_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var7_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var7_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var7_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var8_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var8_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var8_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var9_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var9_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var9_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var10_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var10_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var10_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var11_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var11_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var11_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var12_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var12_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var12_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var13_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var13_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var13_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var14_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var14_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var14_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var15_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var15_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var15_bits; // @[systolic_array.scala 108:28]
  wire  Line_5_var16_ready; // @[systolic_array.scala 108:28]
  wire  Line_5_var16_valid; // @[systolic_array.scala 108:28]
  wire [31:0] Line_5_var16_bits; // @[systolic_array.scala 108:28]
  wire  Line_6_clock; // @[systolic_array.scala 109:28]
  wire  Line_6_reset; // @[systolic_array.scala 109:28]
  wire  Line_6_var0_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var0_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var0_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var1_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var1_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var1_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var2_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var2_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var2_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var3_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var3_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var3_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var4_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var4_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var4_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var5_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var5_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var5_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var6_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var6_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var6_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var7_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var7_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var7_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var8_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var8_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var8_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var9_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var9_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var9_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var10_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var10_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var10_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var11_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var11_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var11_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var12_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var12_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var12_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var13_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var13_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var13_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var14_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var14_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var14_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var15_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var15_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var15_bits; // @[systolic_array.scala 109:28]
  wire  Line_6_var16_ready; // @[systolic_array.scala 109:28]
  wire  Line_6_var16_valid; // @[systolic_array.scala 109:28]
  wire [31:0] Line_6_var16_bits; // @[systolic_array.scala 109:28]
  wire  Line_7_clock; // @[systolic_array.scala 110:28]
  wire  Line_7_reset; // @[systolic_array.scala 110:28]
  wire  Line_7_var0_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var0_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var0_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var1_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var1_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var1_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var2_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var2_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var2_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var3_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var3_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var3_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var4_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var4_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var4_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var5_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var5_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var5_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var6_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var6_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var6_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var7_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var7_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var7_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var8_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var8_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var8_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var9_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var9_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var9_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var10_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var10_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var10_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var11_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var11_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var11_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var12_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var12_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var12_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var13_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var13_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var13_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var14_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var14_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var14_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var15_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var15_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var15_bits; // @[systolic_array.scala 110:28]
  wire  Line_7_var16_ready; // @[systolic_array.scala 110:28]
  wire  Line_7_var16_valid; // @[systolic_array.scala 110:28]
  wire [31:0] Line_7_var16_bits; // @[systolic_array.scala 110:28]
  PE_line Line_0 ( // @[systolic_array.scala 103:28]
    .clock(Line_0_clock),
    .reset(Line_0_reset),
    .var0_ready(Line_0_var0_ready),
    .var0_valid(Line_0_var0_valid),
    .var0_bits(Line_0_var0_bits),
    .var1_ready(Line_0_var1_ready),
    .var1_valid(Line_0_var1_valid),
    .var1_bits(Line_0_var1_bits),
    .var2_ready(Line_0_var2_ready),
    .var2_valid(Line_0_var2_valid),
    .var2_bits(Line_0_var2_bits),
    .var3_ready(Line_0_var3_ready),
    .var3_valid(Line_0_var3_valid),
    .var3_bits(Line_0_var3_bits),
    .var4_ready(Line_0_var4_ready),
    .var4_valid(Line_0_var4_valid),
    .var4_bits(Line_0_var4_bits),
    .var5_ready(Line_0_var5_ready),
    .var5_valid(Line_0_var5_valid),
    .var5_bits(Line_0_var5_bits),
    .var6_ready(Line_0_var6_ready),
    .var6_valid(Line_0_var6_valid),
    .var6_bits(Line_0_var6_bits),
    .var7_ready(Line_0_var7_ready),
    .var7_valid(Line_0_var7_valid),
    .var7_bits(Line_0_var7_bits),
    .var8_ready(Line_0_var8_ready),
    .var8_valid(Line_0_var8_valid),
    .var8_bits(Line_0_var8_bits),
    .var9_ready(Line_0_var9_ready),
    .var9_valid(Line_0_var9_valid),
    .var9_bits(Line_0_var9_bits),
    .var10_ready(Line_0_var10_ready),
    .var10_valid(Line_0_var10_valid),
    .var10_bits(Line_0_var10_bits),
    .var11_ready(Line_0_var11_ready),
    .var11_valid(Line_0_var11_valid),
    .var11_bits(Line_0_var11_bits),
    .var12_ready(Line_0_var12_ready),
    .var12_valid(Line_0_var12_valid),
    .var12_bits(Line_0_var12_bits),
    .var13_ready(Line_0_var13_ready),
    .var13_valid(Line_0_var13_valid),
    .var13_bits(Line_0_var13_bits),
    .var14_ready(Line_0_var14_ready),
    .var14_valid(Line_0_var14_valid),
    .var14_bits(Line_0_var14_bits),
    .var15_ready(Line_0_var15_ready),
    .var15_valid(Line_0_var15_valid),
    .var15_bits(Line_0_var15_bits),
    .var16_ready(Line_0_var16_ready),
    .var16_valid(Line_0_var16_valid),
    .var16_bits(Line_0_var16_bits)
  );
  PE_line_1 Line_1 ( // @[systolic_array.scala 104:28]
    .clock(Line_1_clock),
    .reset(Line_1_reset),
    .var0_ready(Line_1_var0_ready),
    .var0_valid(Line_1_var0_valid),
    .var0_bits(Line_1_var0_bits),
    .var1_ready(Line_1_var1_ready),
    .var1_valid(Line_1_var1_valid),
    .var1_bits(Line_1_var1_bits),
    .var2_ready(Line_1_var2_ready),
    .var2_valid(Line_1_var2_valid),
    .var2_bits(Line_1_var2_bits),
    .var3_ready(Line_1_var3_ready),
    .var3_valid(Line_1_var3_valid),
    .var3_bits(Line_1_var3_bits),
    .var4_ready(Line_1_var4_ready),
    .var4_valid(Line_1_var4_valid),
    .var4_bits(Line_1_var4_bits),
    .var5_ready(Line_1_var5_ready),
    .var5_valid(Line_1_var5_valid),
    .var5_bits(Line_1_var5_bits),
    .var6_ready(Line_1_var6_ready),
    .var6_valid(Line_1_var6_valid),
    .var6_bits(Line_1_var6_bits),
    .var7_ready(Line_1_var7_ready),
    .var7_valid(Line_1_var7_valid),
    .var7_bits(Line_1_var7_bits),
    .var8_ready(Line_1_var8_ready),
    .var8_valid(Line_1_var8_valid),
    .var8_bits(Line_1_var8_bits),
    .var9_ready(Line_1_var9_ready),
    .var9_valid(Line_1_var9_valid),
    .var9_bits(Line_1_var9_bits),
    .var10_ready(Line_1_var10_ready),
    .var10_valid(Line_1_var10_valid),
    .var10_bits(Line_1_var10_bits),
    .var11_ready(Line_1_var11_ready),
    .var11_valid(Line_1_var11_valid),
    .var11_bits(Line_1_var11_bits),
    .var12_ready(Line_1_var12_ready),
    .var12_valid(Line_1_var12_valid),
    .var12_bits(Line_1_var12_bits),
    .var13_ready(Line_1_var13_ready),
    .var13_valid(Line_1_var13_valid),
    .var13_bits(Line_1_var13_bits),
    .var14_ready(Line_1_var14_ready),
    .var14_valid(Line_1_var14_valid),
    .var14_bits(Line_1_var14_bits),
    .var15_ready(Line_1_var15_ready),
    .var15_valid(Line_1_var15_valid),
    .var15_bits(Line_1_var15_bits),
    .var16_ready(Line_1_var16_ready),
    .var16_valid(Line_1_var16_valid),
    .var16_bits(Line_1_var16_bits)
  );
  PE_line_2 Line_2 ( // @[systolic_array.scala 105:28]
    .clock(Line_2_clock),
    .reset(Line_2_reset),
    .var0_ready(Line_2_var0_ready),
    .var0_valid(Line_2_var0_valid),
    .var0_bits(Line_2_var0_bits),
    .var1_ready(Line_2_var1_ready),
    .var1_valid(Line_2_var1_valid),
    .var1_bits(Line_2_var1_bits),
    .var2_ready(Line_2_var2_ready),
    .var2_valid(Line_2_var2_valid),
    .var2_bits(Line_2_var2_bits),
    .var3_ready(Line_2_var3_ready),
    .var3_valid(Line_2_var3_valid),
    .var3_bits(Line_2_var3_bits),
    .var4_ready(Line_2_var4_ready),
    .var4_valid(Line_2_var4_valid),
    .var4_bits(Line_2_var4_bits),
    .var5_ready(Line_2_var5_ready),
    .var5_valid(Line_2_var5_valid),
    .var5_bits(Line_2_var5_bits),
    .var6_ready(Line_2_var6_ready),
    .var6_valid(Line_2_var6_valid),
    .var6_bits(Line_2_var6_bits),
    .var7_ready(Line_2_var7_ready),
    .var7_valid(Line_2_var7_valid),
    .var7_bits(Line_2_var7_bits),
    .var8_ready(Line_2_var8_ready),
    .var8_valid(Line_2_var8_valid),
    .var8_bits(Line_2_var8_bits),
    .var9_ready(Line_2_var9_ready),
    .var9_valid(Line_2_var9_valid),
    .var9_bits(Line_2_var9_bits),
    .var10_ready(Line_2_var10_ready),
    .var10_valid(Line_2_var10_valid),
    .var10_bits(Line_2_var10_bits),
    .var11_ready(Line_2_var11_ready),
    .var11_valid(Line_2_var11_valid),
    .var11_bits(Line_2_var11_bits),
    .var12_ready(Line_2_var12_ready),
    .var12_valid(Line_2_var12_valid),
    .var12_bits(Line_2_var12_bits),
    .var13_ready(Line_2_var13_ready),
    .var13_valid(Line_2_var13_valid),
    .var13_bits(Line_2_var13_bits),
    .var14_ready(Line_2_var14_ready),
    .var14_valid(Line_2_var14_valid),
    .var14_bits(Line_2_var14_bits),
    .var15_ready(Line_2_var15_ready),
    .var15_valid(Line_2_var15_valid),
    .var15_bits(Line_2_var15_bits),
    .var16_ready(Line_2_var16_ready),
    .var16_valid(Line_2_var16_valid),
    .var16_bits(Line_2_var16_bits)
  );
  PE_line_3 Line_3 ( // @[systolic_array.scala 106:28]
    .clock(Line_3_clock),
    .reset(Line_3_reset),
    .var0_ready(Line_3_var0_ready),
    .var0_valid(Line_3_var0_valid),
    .var0_bits(Line_3_var0_bits),
    .var1_ready(Line_3_var1_ready),
    .var1_valid(Line_3_var1_valid),
    .var1_bits(Line_3_var1_bits),
    .var2_ready(Line_3_var2_ready),
    .var2_valid(Line_3_var2_valid),
    .var2_bits(Line_3_var2_bits),
    .var3_ready(Line_3_var3_ready),
    .var3_valid(Line_3_var3_valid),
    .var3_bits(Line_3_var3_bits),
    .var4_ready(Line_3_var4_ready),
    .var4_valid(Line_3_var4_valid),
    .var4_bits(Line_3_var4_bits),
    .var5_ready(Line_3_var5_ready),
    .var5_valid(Line_3_var5_valid),
    .var5_bits(Line_3_var5_bits),
    .var6_ready(Line_3_var6_ready),
    .var6_valid(Line_3_var6_valid),
    .var6_bits(Line_3_var6_bits),
    .var7_ready(Line_3_var7_ready),
    .var7_valid(Line_3_var7_valid),
    .var7_bits(Line_3_var7_bits),
    .var8_ready(Line_3_var8_ready),
    .var8_valid(Line_3_var8_valid),
    .var8_bits(Line_3_var8_bits),
    .var9_ready(Line_3_var9_ready),
    .var9_valid(Line_3_var9_valid),
    .var9_bits(Line_3_var9_bits),
    .var10_ready(Line_3_var10_ready),
    .var10_valid(Line_3_var10_valid),
    .var10_bits(Line_3_var10_bits),
    .var11_ready(Line_3_var11_ready),
    .var11_valid(Line_3_var11_valid),
    .var11_bits(Line_3_var11_bits),
    .var12_ready(Line_3_var12_ready),
    .var12_valid(Line_3_var12_valid),
    .var12_bits(Line_3_var12_bits),
    .var13_ready(Line_3_var13_ready),
    .var13_valid(Line_3_var13_valid),
    .var13_bits(Line_3_var13_bits),
    .var14_ready(Line_3_var14_ready),
    .var14_valid(Line_3_var14_valid),
    .var14_bits(Line_3_var14_bits),
    .var15_ready(Line_3_var15_ready),
    .var15_valid(Line_3_var15_valid),
    .var15_bits(Line_3_var15_bits),
    .var16_ready(Line_3_var16_ready),
    .var16_valid(Line_3_var16_valid),
    .var16_bits(Line_3_var16_bits)
  );
  PE_line_4 Line_4 ( // @[systolic_array.scala 107:28]
    .clock(Line_4_clock),
    .reset(Line_4_reset),
    .var0_ready(Line_4_var0_ready),
    .var0_valid(Line_4_var0_valid),
    .var0_bits(Line_4_var0_bits),
    .var1_ready(Line_4_var1_ready),
    .var1_valid(Line_4_var1_valid),
    .var1_bits(Line_4_var1_bits),
    .var2_ready(Line_4_var2_ready),
    .var2_valid(Line_4_var2_valid),
    .var2_bits(Line_4_var2_bits),
    .var3_ready(Line_4_var3_ready),
    .var3_valid(Line_4_var3_valid),
    .var3_bits(Line_4_var3_bits),
    .var4_ready(Line_4_var4_ready),
    .var4_valid(Line_4_var4_valid),
    .var4_bits(Line_4_var4_bits),
    .var5_ready(Line_4_var5_ready),
    .var5_valid(Line_4_var5_valid),
    .var5_bits(Line_4_var5_bits),
    .var6_ready(Line_4_var6_ready),
    .var6_valid(Line_4_var6_valid),
    .var6_bits(Line_4_var6_bits),
    .var7_ready(Line_4_var7_ready),
    .var7_valid(Line_4_var7_valid),
    .var7_bits(Line_4_var7_bits),
    .var8_ready(Line_4_var8_ready),
    .var8_valid(Line_4_var8_valid),
    .var8_bits(Line_4_var8_bits),
    .var9_ready(Line_4_var9_ready),
    .var9_valid(Line_4_var9_valid),
    .var9_bits(Line_4_var9_bits),
    .var10_ready(Line_4_var10_ready),
    .var10_valid(Line_4_var10_valid),
    .var10_bits(Line_4_var10_bits),
    .var11_ready(Line_4_var11_ready),
    .var11_valid(Line_4_var11_valid),
    .var11_bits(Line_4_var11_bits),
    .var12_ready(Line_4_var12_ready),
    .var12_valid(Line_4_var12_valid),
    .var12_bits(Line_4_var12_bits),
    .var13_ready(Line_4_var13_ready),
    .var13_valid(Line_4_var13_valid),
    .var13_bits(Line_4_var13_bits),
    .var14_ready(Line_4_var14_ready),
    .var14_valid(Line_4_var14_valid),
    .var14_bits(Line_4_var14_bits),
    .var15_ready(Line_4_var15_ready),
    .var15_valid(Line_4_var15_valid),
    .var15_bits(Line_4_var15_bits),
    .var16_ready(Line_4_var16_ready),
    .var16_valid(Line_4_var16_valid),
    .var16_bits(Line_4_var16_bits)
  );
  PE_line_5 Line_5 ( // @[systolic_array.scala 108:28]
    .clock(Line_5_clock),
    .reset(Line_5_reset),
    .var0_ready(Line_5_var0_ready),
    .var0_valid(Line_5_var0_valid),
    .var0_bits(Line_5_var0_bits),
    .var1_ready(Line_5_var1_ready),
    .var1_valid(Line_5_var1_valid),
    .var1_bits(Line_5_var1_bits),
    .var2_ready(Line_5_var2_ready),
    .var2_valid(Line_5_var2_valid),
    .var2_bits(Line_5_var2_bits),
    .var3_ready(Line_5_var3_ready),
    .var3_valid(Line_5_var3_valid),
    .var3_bits(Line_5_var3_bits),
    .var4_ready(Line_5_var4_ready),
    .var4_valid(Line_5_var4_valid),
    .var4_bits(Line_5_var4_bits),
    .var5_ready(Line_5_var5_ready),
    .var5_valid(Line_5_var5_valid),
    .var5_bits(Line_5_var5_bits),
    .var6_ready(Line_5_var6_ready),
    .var6_valid(Line_5_var6_valid),
    .var6_bits(Line_5_var6_bits),
    .var7_ready(Line_5_var7_ready),
    .var7_valid(Line_5_var7_valid),
    .var7_bits(Line_5_var7_bits),
    .var8_ready(Line_5_var8_ready),
    .var8_valid(Line_5_var8_valid),
    .var8_bits(Line_5_var8_bits),
    .var9_ready(Line_5_var9_ready),
    .var9_valid(Line_5_var9_valid),
    .var9_bits(Line_5_var9_bits),
    .var10_ready(Line_5_var10_ready),
    .var10_valid(Line_5_var10_valid),
    .var10_bits(Line_5_var10_bits),
    .var11_ready(Line_5_var11_ready),
    .var11_valid(Line_5_var11_valid),
    .var11_bits(Line_5_var11_bits),
    .var12_ready(Line_5_var12_ready),
    .var12_valid(Line_5_var12_valid),
    .var12_bits(Line_5_var12_bits),
    .var13_ready(Line_5_var13_ready),
    .var13_valid(Line_5_var13_valid),
    .var13_bits(Line_5_var13_bits),
    .var14_ready(Line_5_var14_ready),
    .var14_valid(Line_5_var14_valid),
    .var14_bits(Line_5_var14_bits),
    .var15_ready(Line_5_var15_ready),
    .var15_valid(Line_5_var15_valid),
    .var15_bits(Line_5_var15_bits),
    .var16_ready(Line_5_var16_ready),
    .var16_valid(Line_5_var16_valid),
    .var16_bits(Line_5_var16_bits)
  );
  PE_line_6 Line_6 ( // @[systolic_array.scala 109:28]
    .clock(Line_6_clock),
    .reset(Line_6_reset),
    .var0_ready(Line_6_var0_ready),
    .var0_valid(Line_6_var0_valid),
    .var0_bits(Line_6_var0_bits),
    .var1_ready(Line_6_var1_ready),
    .var1_valid(Line_6_var1_valid),
    .var1_bits(Line_6_var1_bits),
    .var2_ready(Line_6_var2_ready),
    .var2_valid(Line_6_var2_valid),
    .var2_bits(Line_6_var2_bits),
    .var3_ready(Line_6_var3_ready),
    .var3_valid(Line_6_var3_valid),
    .var3_bits(Line_6_var3_bits),
    .var4_ready(Line_6_var4_ready),
    .var4_valid(Line_6_var4_valid),
    .var4_bits(Line_6_var4_bits),
    .var5_ready(Line_6_var5_ready),
    .var5_valid(Line_6_var5_valid),
    .var5_bits(Line_6_var5_bits),
    .var6_ready(Line_6_var6_ready),
    .var6_valid(Line_6_var6_valid),
    .var6_bits(Line_6_var6_bits),
    .var7_ready(Line_6_var7_ready),
    .var7_valid(Line_6_var7_valid),
    .var7_bits(Line_6_var7_bits),
    .var8_ready(Line_6_var8_ready),
    .var8_valid(Line_6_var8_valid),
    .var8_bits(Line_6_var8_bits),
    .var9_ready(Line_6_var9_ready),
    .var9_valid(Line_6_var9_valid),
    .var9_bits(Line_6_var9_bits),
    .var10_ready(Line_6_var10_ready),
    .var10_valid(Line_6_var10_valid),
    .var10_bits(Line_6_var10_bits),
    .var11_ready(Line_6_var11_ready),
    .var11_valid(Line_6_var11_valid),
    .var11_bits(Line_6_var11_bits),
    .var12_ready(Line_6_var12_ready),
    .var12_valid(Line_6_var12_valid),
    .var12_bits(Line_6_var12_bits),
    .var13_ready(Line_6_var13_ready),
    .var13_valid(Line_6_var13_valid),
    .var13_bits(Line_6_var13_bits),
    .var14_ready(Line_6_var14_ready),
    .var14_valid(Line_6_var14_valid),
    .var14_bits(Line_6_var14_bits),
    .var15_ready(Line_6_var15_ready),
    .var15_valid(Line_6_var15_valid),
    .var15_bits(Line_6_var15_bits),
    .var16_ready(Line_6_var16_ready),
    .var16_valid(Line_6_var16_valid),
    .var16_bits(Line_6_var16_bits)
  );
  PE_line_7 Line_7 ( // @[systolic_array.scala 110:28]
    .clock(Line_7_clock),
    .reset(Line_7_reset),
    .var0_ready(Line_7_var0_ready),
    .var0_valid(Line_7_var0_valid),
    .var0_bits(Line_7_var0_bits),
    .var1_ready(Line_7_var1_ready),
    .var1_valid(Line_7_var1_valid),
    .var1_bits(Line_7_var1_bits),
    .var2_ready(Line_7_var2_ready),
    .var2_valid(Line_7_var2_valid),
    .var2_bits(Line_7_var2_bits),
    .var3_ready(Line_7_var3_ready),
    .var3_valid(Line_7_var3_valid),
    .var3_bits(Line_7_var3_bits),
    .var4_ready(Line_7_var4_ready),
    .var4_valid(Line_7_var4_valid),
    .var4_bits(Line_7_var4_bits),
    .var5_ready(Line_7_var5_ready),
    .var5_valid(Line_7_var5_valid),
    .var5_bits(Line_7_var5_bits),
    .var6_ready(Line_7_var6_ready),
    .var6_valid(Line_7_var6_valid),
    .var6_bits(Line_7_var6_bits),
    .var7_ready(Line_7_var7_ready),
    .var7_valid(Line_7_var7_valid),
    .var7_bits(Line_7_var7_bits),
    .var8_ready(Line_7_var8_ready),
    .var8_valid(Line_7_var8_valid),
    .var8_bits(Line_7_var8_bits),
    .var9_ready(Line_7_var9_ready),
    .var9_valid(Line_7_var9_valid),
    .var9_bits(Line_7_var9_bits),
    .var10_ready(Line_7_var10_ready),
    .var10_valid(Line_7_var10_valid),
    .var10_bits(Line_7_var10_bits),
    .var11_ready(Line_7_var11_ready),
    .var11_valid(Line_7_var11_valid),
    .var11_bits(Line_7_var11_bits),
    .var12_ready(Line_7_var12_ready),
    .var12_valid(Line_7_var12_valid),
    .var12_bits(Line_7_var12_bits),
    .var13_ready(Line_7_var13_ready),
    .var13_valid(Line_7_var13_valid),
    .var13_bits(Line_7_var13_bits),
    .var14_ready(Line_7_var14_ready),
    .var14_valid(Line_7_var14_valid),
    .var14_bits(Line_7_var14_bits),
    .var15_ready(Line_7_var15_ready),
    .var15_valid(Line_7_var15_valid),
    .var15_bits(Line_7_var15_bits),
    .var16_ready(Line_7_var16_ready),
    .var16_valid(Line_7_var16_valid),
    .var16_bits(Line_7_var16_bits)
  );
  assign var17_ready = Line_0_var0_ready; // @[systolic_array.scala 111:21]
  assign var18_ready = Line_1_var0_ready; // @[systolic_array.scala 112:21]
  assign var19_ready = Line_2_var0_ready; // @[systolic_array.scala 113:21]
  assign var20_ready = Line_3_var0_ready; // @[systolic_array.scala 114:21]
  assign var21_ready = Line_4_var0_ready; // @[systolic_array.scala 115:21]
  assign var22_ready = Line_5_var0_ready; // @[systolic_array.scala 116:21]
  assign var23_ready = Line_6_var0_ready; // @[systolic_array.scala 117:21]
  assign var24_ready = Line_7_var0_ready; // @[systolic_array.scala 118:21]
  assign var25_ready = Line_0_var1_ready; // @[systolic_array.scala 119:21]
  assign var26_ready = Line_0_var2_ready; // @[systolic_array.scala 120:21]
  assign var27_ready = Line_0_var3_ready; // @[systolic_array.scala 121:21]
  assign var28_ready = Line_0_var4_ready; // @[systolic_array.scala 122:21]
  assign var29_ready = Line_0_var5_ready; // @[systolic_array.scala 123:21]
  assign var30_ready = Line_0_var6_ready; // @[systolic_array.scala 124:21]
  assign var31_ready = Line_0_var7_ready; // @[systolic_array.scala 125:21]
  assign var32_ready = Line_0_var8_ready; // @[systolic_array.scala 126:21]
  assign var33_valid = Line_7_var9_valid; // @[systolic_array.scala 183:15]
  assign var33_bits = Line_7_var9_bits; // @[systolic_array.scala 183:15]
  assign var34_valid = Line_7_var10_valid; // @[systolic_array.scala 184:15]
  assign var34_bits = Line_7_var10_bits; // @[systolic_array.scala 184:15]
  assign var35_valid = Line_7_var11_valid; // @[systolic_array.scala 185:15]
  assign var35_bits = Line_7_var11_bits; // @[systolic_array.scala 185:15]
  assign var36_valid = Line_7_var12_valid; // @[systolic_array.scala 186:15]
  assign var36_bits = Line_7_var12_bits; // @[systolic_array.scala 186:15]
  assign var37_valid = Line_7_var13_valid; // @[systolic_array.scala 187:15]
  assign var37_bits = Line_7_var13_bits; // @[systolic_array.scala 187:15]
  assign var38_valid = Line_7_var14_valid; // @[systolic_array.scala 188:15]
  assign var38_bits = Line_7_var14_bits; // @[systolic_array.scala 188:15]
  assign var39_valid = Line_7_var15_valid; // @[systolic_array.scala 189:15]
  assign var39_bits = Line_7_var15_bits; // @[systolic_array.scala 189:15]
  assign var40_valid = Line_7_var16_valid; // @[systolic_array.scala 190:15]
  assign var40_bits = Line_7_var16_bits; // @[systolic_array.scala 190:15]
  assign Line_0_clock = clock;
  assign Line_0_reset = reset;
  assign Line_0_var0_valid = var17_valid; // @[systolic_array.scala 111:21]
  assign Line_0_var0_bits = var17_bits; // @[systolic_array.scala 111:21]
  assign Line_0_var1_valid = var25_valid; // @[systolic_array.scala 119:21]
  assign Line_0_var1_bits = var25_bits; // @[systolic_array.scala 119:21]
  assign Line_0_var2_valid = var26_valid; // @[systolic_array.scala 120:21]
  assign Line_0_var2_bits = var26_bits; // @[systolic_array.scala 120:21]
  assign Line_0_var3_valid = var27_valid; // @[systolic_array.scala 121:21]
  assign Line_0_var3_bits = var27_bits; // @[systolic_array.scala 121:21]
  assign Line_0_var4_valid = var28_valid; // @[systolic_array.scala 122:21]
  assign Line_0_var4_bits = var28_bits; // @[systolic_array.scala 122:21]
  assign Line_0_var5_valid = var29_valid; // @[systolic_array.scala 123:21]
  assign Line_0_var5_bits = var29_bits; // @[systolic_array.scala 123:21]
  assign Line_0_var6_valid = var30_valid; // @[systolic_array.scala 124:21]
  assign Line_0_var6_bits = var30_bits; // @[systolic_array.scala 124:21]
  assign Line_0_var7_valid = var31_valid; // @[systolic_array.scala 125:21]
  assign Line_0_var7_bits = var31_bits; // @[systolic_array.scala 125:21]
  assign Line_0_var8_valid = var32_valid; // @[systolic_array.scala 126:21]
  assign Line_0_var8_bits = var32_bits; // @[systolic_array.scala 126:21]
  assign Line_0_var9_ready = Line_1_var1_ready; // @[systolic_array.scala 127:21]
  assign Line_0_var10_ready = Line_1_var2_ready; // @[systolic_array.scala 128:21]
  assign Line_0_var11_ready = Line_1_var3_ready; // @[systolic_array.scala 129:21]
  assign Line_0_var12_ready = Line_1_var4_ready; // @[systolic_array.scala 130:21]
  assign Line_0_var13_ready = Line_1_var5_ready; // @[systolic_array.scala 131:21]
  assign Line_0_var14_ready = Line_1_var6_ready; // @[systolic_array.scala 132:21]
  assign Line_0_var15_ready = Line_1_var7_ready; // @[systolic_array.scala 133:21]
  assign Line_0_var16_ready = Line_1_var8_ready; // @[systolic_array.scala 134:21]
  assign Line_1_clock = clock;
  assign Line_1_reset = reset;
  assign Line_1_var0_valid = var18_valid; // @[systolic_array.scala 112:21]
  assign Line_1_var0_bits = var18_bits; // @[systolic_array.scala 112:21]
  assign Line_1_var1_valid = Line_0_var9_valid; // @[systolic_array.scala 127:21]
  assign Line_1_var1_bits = Line_0_var9_bits; // @[systolic_array.scala 127:21]
  assign Line_1_var2_valid = Line_0_var10_valid; // @[systolic_array.scala 128:21]
  assign Line_1_var2_bits = Line_0_var10_bits; // @[systolic_array.scala 128:21]
  assign Line_1_var3_valid = Line_0_var11_valid; // @[systolic_array.scala 129:21]
  assign Line_1_var3_bits = Line_0_var11_bits; // @[systolic_array.scala 129:21]
  assign Line_1_var4_valid = Line_0_var12_valid; // @[systolic_array.scala 130:21]
  assign Line_1_var4_bits = Line_0_var12_bits; // @[systolic_array.scala 130:21]
  assign Line_1_var5_valid = Line_0_var13_valid; // @[systolic_array.scala 131:21]
  assign Line_1_var5_bits = Line_0_var13_bits; // @[systolic_array.scala 131:21]
  assign Line_1_var6_valid = Line_0_var14_valid; // @[systolic_array.scala 132:21]
  assign Line_1_var6_bits = Line_0_var14_bits; // @[systolic_array.scala 132:21]
  assign Line_1_var7_valid = Line_0_var15_valid; // @[systolic_array.scala 133:21]
  assign Line_1_var7_bits = Line_0_var15_bits; // @[systolic_array.scala 133:21]
  assign Line_1_var8_valid = Line_0_var16_valid; // @[systolic_array.scala 134:21]
  assign Line_1_var8_bits = Line_0_var16_bits; // @[systolic_array.scala 134:21]
  assign Line_1_var9_ready = Line_2_var1_ready; // @[systolic_array.scala 135:21]
  assign Line_1_var10_ready = Line_2_var2_ready; // @[systolic_array.scala 136:21]
  assign Line_1_var11_ready = Line_2_var3_ready; // @[systolic_array.scala 137:21]
  assign Line_1_var12_ready = Line_2_var4_ready; // @[systolic_array.scala 138:21]
  assign Line_1_var13_ready = Line_2_var5_ready; // @[systolic_array.scala 139:21]
  assign Line_1_var14_ready = Line_2_var6_ready; // @[systolic_array.scala 140:21]
  assign Line_1_var15_ready = Line_2_var7_ready; // @[systolic_array.scala 141:21]
  assign Line_1_var16_ready = Line_2_var8_ready; // @[systolic_array.scala 142:21]
  assign Line_2_clock = clock;
  assign Line_2_reset = reset;
  assign Line_2_var0_valid = var19_valid; // @[systolic_array.scala 113:21]
  assign Line_2_var0_bits = var19_bits; // @[systolic_array.scala 113:21]
  assign Line_2_var1_valid = Line_1_var9_valid; // @[systolic_array.scala 135:21]
  assign Line_2_var1_bits = Line_1_var9_bits; // @[systolic_array.scala 135:21]
  assign Line_2_var2_valid = Line_1_var10_valid; // @[systolic_array.scala 136:21]
  assign Line_2_var2_bits = Line_1_var10_bits; // @[systolic_array.scala 136:21]
  assign Line_2_var3_valid = Line_1_var11_valid; // @[systolic_array.scala 137:21]
  assign Line_2_var3_bits = Line_1_var11_bits; // @[systolic_array.scala 137:21]
  assign Line_2_var4_valid = Line_1_var12_valid; // @[systolic_array.scala 138:21]
  assign Line_2_var4_bits = Line_1_var12_bits; // @[systolic_array.scala 138:21]
  assign Line_2_var5_valid = Line_1_var13_valid; // @[systolic_array.scala 139:21]
  assign Line_2_var5_bits = Line_1_var13_bits; // @[systolic_array.scala 139:21]
  assign Line_2_var6_valid = Line_1_var14_valid; // @[systolic_array.scala 140:21]
  assign Line_2_var6_bits = Line_1_var14_bits; // @[systolic_array.scala 140:21]
  assign Line_2_var7_valid = Line_1_var15_valid; // @[systolic_array.scala 141:21]
  assign Line_2_var7_bits = Line_1_var15_bits; // @[systolic_array.scala 141:21]
  assign Line_2_var8_valid = Line_1_var16_valid; // @[systolic_array.scala 142:21]
  assign Line_2_var8_bits = Line_1_var16_bits; // @[systolic_array.scala 142:21]
  assign Line_2_var9_ready = Line_3_var1_ready; // @[systolic_array.scala 143:21]
  assign Line_2_var10_ready = Line_3_var2_ready; // @[systolic_array.scala 144:21]
  assign Line_2_var11_ready = Line_3_var3_ready; // @[systolic_array.scala 145:21]
  assign Line_2_var12_ready = Line_3_var4_ready; // @[systolic_array.scala 146:21]
  assign Line_2_var13_ready = Line_3_var5_ready; // @[systolic_array.scala 147:21]
  assign Line_2_var14_ready = Line_3_var6_ready; // @[systolic_array.scala 148:21]
  assign Line_2_var15_ready = Line_3_var7_ready; // @[systolic_array.scala 149:21]
  assign Line_2_var16_ready = Line_3_var8_ready; // @[systolic_array.scala 150:21]
  assign Line_3_clock = clock;
  assign Line_3_reset = reset;
  assign Line_3_var0_valid = var20_valid; // @[systolic_array.scala 114:21]
  assign Line_3_var0_bits = var20_bits; // @[systolic_array.scala 114:21]
  assign Line_3_var1_valid = Line_2_var9_valid; // @[systolic_array.scala 143:21]
  assign Line_3_var1_bits = Line_2_var9_bits; // @[systolic_array.scala 143:21]
  assign Line_3_var2_valid = Line_2_var10_valid; // @[systolic_array.scala 144:21]
  assign Line_3_var2_bits = Line_2_var10_bits; // @[systolic_array.scala 144:21]
  assign Line_3_var3_valid = Line_2_var11_valid; // @[systolic_array.scala 145:21]
  assign Line_3_var3_bits = Line_2_var11_bits; // @[systolic_array.scala 145:21]
  assign Line_3_var4_valid = Line_2_var12_valid; // @[systolic_array.scala 146:21]
  assign Line_3_var4_bits = Line_2_var12_bits; // @[systolic_array.scala 146:21]
  assign Line_3_var5_valid = Line_2_var13_valid; // @[systolic_array.scala 147:21]
  assign Line_3_var5_bits = Line_2_var13_bits; // @[systolic_array.scala 147:21]
  assign Line_3_var6_valid = Line_2_var14_valid; // @[systolic_array.scala 148:21]
  assign Line_3_var6_bits = Line_2_var14_bits; // @[systolic_array.scala 148:21]
  assign Line_3_var7_valid = Line_2_var15_valid; // @[systolic_array.scala 149:21]
  assign Line_3_var7_bits = Line_2_var15_bits; // @[systolic_array.scala 149:21]
  assign Line_3_var8_valid = Line_2_var16_valid; // @[systolic_array.scala 150:21]
  assign Line_3_var8_bits = Line_2_var16_bits; // @[systolic_array.scala 150:21]
  assign Line_3_var9_ready = Line_4_var1_ready; // @[systolic_array.scala 151:21]
  assign Line_3_var10_ready = Line_4_var2_ready; // @[systolic_array.scala 152:21]
  assign Line_3_var11_ready = Line_4_var3_ready; // @[systolic_array.scala 153:21]
  assign Line_3_var12_ready = Line_4_var4_ready; // @[systolic_array.scala 154:21]
  assign Line_3_var13_ready = Line_4_var5_ready; // @[systolic_array.scala 155:21]
  assign Line_3_var14_ready = Line_4_var6_ready; // @[systolic_array.scala 156:21]
  assign Line_3_var15_ready = Line_4_var7_ready; // @[systolic_array.scala 157:21]
  assign Line_3_var16_ready = Line_4_var8_ready; // @[systolic_array.scala 158:21]
  assign Line_4_clock = clock;
  assign Line_4_reset = reset;
  assign Line_4_var0_valid = var21_valid; // @[systolic_array.scala 115:21]
  assign Line_4_var0_bits = var21_bits; // @[systolic_array.scala 115:21]
  assign Line_4_var1_valid = Line_3_var9_valid; // @[systolic_array.scala 151:21]
  assign Line_4_var1_bits = Line_3_var9_bits; // @[systolic_array.scala 151:21]
  assign Line_4_var2_valid = Line_3_var10_valid; // @[systolic_array.scala 152:21]
  assign Line_4_var2_bits = Line_3_var10_bits; // @[systolic_array.scala 152:21]
  assign Line_4_var3_valid = Line_3_var11_valid; // @[systolic_array.scala 153:21]
  assign Line_4_var3_bits = Line_3_var11_bits; // @[systolic_array.scala 153:21]
  assign Line_4_var4_valid = Line_3_var12_valid; // @[systolic_array.scala 154:21]
  assign Line_4_var4_bits = Line_3_var12_bits; // @[systolic_array.scala 154:21]
  assign Line_4_var5_valid = Line_3_var13_valid; // @[systolic_array.scala 155:21]
  assign Line_4_var5_bits = Line_3_var13_bits; // @[systolic_array.scala 155:21]
  assign Line_4_var6_valid = Line_3_var14_valid; // @[systolic_array.scala 156:21]
  assign Line_4_var6_bits = Line_3_var14_bits; // @[systolic_array.scala 156:21]
  assign Line_4_var7_valid = Line_3_var15_valid; // @[systolic_array.scala 157:21]
  assign Line_4_var7_bits = Line_3_var15_bits; // @[systolic_array.scala 157:21]
  assign Line_4_var8_valid = Line_3_var16_valid; // @[systolic_array.scala 158:21]
  assign Line_4_var8_bits = Line_3_var16_bits; // @[systolic_array.scala 158:21]
  assign Line_4_var9_ready = Line_5_var1_ready; // @[systolic_array.scala 159:21]
  assign Line_4_var10_ready = Line_5_var2_ready; // @[systolic_array.scala 160:21]
  assign Line_4_var11_ready = Line_5_var3_ready; // @[systolic_array.scala 161:21]
  assign Line_4_var12_ready = Line_5_var4_ready; // @[systolic_array.scala 162:21]
  assign Line_4_var13_ready = Line_5_var5_ready; // @[systolic_array.scala 163:21]
  assign Line_4_var14_ready = Line_5_var6_ready; // @[systolic_array.scala 164:21]
  assign Line_4_var15_ready = Line_5_var7_ready; // @[systolic_array.scala 165:21]
  assign Line_4_var16_ready = Line_5_var8_ready; // @[systolic_array.scala 166:21]
  assign Line_5_clock = clock;
  assign Line_5_reset = reset;
  assign Line_5_var0_valid = var22_valid; // @[systolic_array.scala 116:21]
  assign Line_5_var0_bits = var22_bits; // @[systolic_array.scala 116:21]
  assign Line_5_var1_valid = Line_4_var9_valid; // @[systolic_array.scala 159:21]
  assign Line_5_var1_bits = Line_4_var9_bits; // @[systolic_array.scala 159:21]
  assign Line_5_var2_valid = Line_4_var10_valid; // @[systolic_array.scala 160:21]
  assign Line_5_var2_bits = Line_4_var10_bits; // @[systolic_array.scala 160:21]
  assign Line_5_var3_valid = Line_4_var11_valid; // @[systolic_array.scala 161:21]
  assign Line_5_var3_bits = Line_4_var11_bits; // @[systolic_array.scala 161:21]
  assign Line_5_var4_valid = Line_4_var12_valid; // @[systolic_array.scala 162:21]
  assign Line_5_var4_bits = Line_4_var12_bits; // @[systolic_array.scala 162:21]
  assign Line_5_var5_valid = Line_4_var13_valid; // @[systolic_array.scala 163:21]
  assign Line_5_var5_bits = Line_4_var13_bits; // @[systolic_array.scala 163:21]
  assign Line_5_var6_valid = Line_4_var14_valid; // @[systolic_array.scala 164:21]
  assign Line_5_var6_bits = Line_4_var14_bits; // @[systolic_array.scala 164:21]
  assign Line_5_var7_valid = Line_4_var15_valid; // @[systolic_array.scala 165:21]
  assign Line_5_var7_bits = Line_4_var15_bits; // @[systolic_array.scala 165:21]
  assign Line_5_var8_valid = Line_4_var16_valid; // @[systolic_array.scala 166:21]
  assign Line_5_var8_bits = Line_4_var16_bits; // @[systolic_array.scala 166:21]
  assign Line_5_var9_ready = Line_6_var1_ready; // @[systolic_array.scala 167:21]
  assign Line_5_var10_ready = Line_6_var2_ready; // @[systolic_array.scala 168:21]
  assign Line_5_var11_ready = Line_6_var3_ready; // @[systolic_array.scala 169:21]
  assign Line_5_var12_ready = Line_6_var4_ready; // @[systolic_array.scala 170:21]
  assign Line_5_var13_ready = Line_6_var5_ready; // @[systolic_array.scala 171:21]
  assign Line_5_var14_ready = Line_6_var6_ready; // @[systolic_array.scala 172:21]
  assign Line_5_var15_ready = Line_6_var7_ready; // @[systolic_array.scala 173:21]
  assign Line_5_var16_ready = Line_6_var8_ready; // @[systolic_array.scala 174:21]
  assign Line_6_clock = clock;
  assign Line_6_reset = reset;
  assign Line_6_var0_valid = var23_valid; // @[systolic_array.scala 117:21]
  assign Line_6_var0_bits = var23_bits; // @[systolic_array.scala 117:21]
  assign Line_6_var1_valid = Line_5_var9_valid; // @[systolic_array.scala 167:21]
  assign Line_6_var1_bits = Line_5_var9_bits; // @[systolic_array.scala 167:21]
  assign Line_6_var2_valid = Line_5_var10_valid; // @[systolic_array.scala 168:21]
  assign Line_6_var2_bits = Line_5_var10_bits; // @[systolic_array.scala 168:21]
  assign Line_6_var3_valid = Line_5_var11_valid; // @[systolic_array.scala 169:21]
  assign Line_6_var3_bits = Line_5_var11_bits; // @[systolic_array.scala 169:21]
  assign Line_6_var4_valid = Line_5_var12_valid; // @[systolic_array.scala 170:21]
  assign Line_6_var4_bits = Line_5_var12_bits; // @[systolic_array.scala 170:21]
  assign Line_6_var5_valid = Line_5_var13_valid; // @[systolic_array.scala 171:21]
  assign Line_6_var5_bits = Line_5_var13_bits; // @[systolic_array.scala 171:21]
  assign Line_6_var6_valid = Line_5_var14_valid; // @[systolic_array.scala 172:21]
  assign Line_6_var6_bits = Line_5_var14_bits; // @[systolic_array.scala 172:21]
  assign Line_6_var7_valid = Line_5_var15_valid; // @[systolic_array.scala 173:21]
  assign Line_6_var7_bits = Line_5_var15_bits; // @[systolic_array.scala 173:21]
  assign Line_6_var8_valid = Line_5_var16_valid; // @[systolic_array.scala 174:21]
  assign Line_6_var8_bits = Line_5_var16_bits; // @[systolic_array.scala 174:21]
  assign Line_6_var9_ready = Line_7_var1_ready; // @[systolic_array.scala 175:21]
  assign Line_6_var10_ready = Line_7_var2_ready; // @[systolic_array.scala 176:21]
  assign Line_6_var11_ready = Line_7_var3_ready; // @[systolic_array.scala 177:21]
  assign Line_6_var12_ready = Line_7_var4_ready; // @[systolic_array.scala 178:21]
  assign Line_6_var13_ready = Line_7_var5_ready; // @[systolic_array.scala 179:21]
  assign Line_6_var14_ready = Line_7_var6_ready; // @[systolic_array.scala 180:21]
  assign Line_6_var15_ready = Line_7_var7_ready; // @[systolic_array.scala 181:21]
  assign Line_6_var16_ready = Line_7_var8_ready; // @[systolic_array.scala 182:21]
  assign Line_7_clock = clock;
  assign Line_7_reset = reset;
  assign Line_7_var0_valid = var24_valid; // @[systolic_array.scala 118:21]
  assign Line_7_var0_bits = var24_bits; // @[systolic_array.scala 118:21]
  assign Line_7_var1_valid = Line_6_var9_valid; // @[systolic_array.scala 175:21]
  assign Line_7_var1_bits = Line_6_var9_bits; // @[systolic_array.scala 175:21]
  assign Line_7_var2_valid = Line_6_var10_valid; // @[systolic_array.scala 176:21]
  assign Line_7_var2_bits = Line_6_var10_bits; // @[systolic_array.scala 176:21]
  assign Line_7_var3_valid = Line_6_var11_valid; // @[systolic_array.scala 177:21]
  assign Line_7_var3_bits = Line_6_var11_bits; // @[systolic_array.scala 177:21]
  assign Line_7_var4_valid = Line_6_var12_valid; // @[systolic_array.scala 178:21]
  assign Line_7_var4_bits = Line_6_var12_bits; // @[systolic_array.scala 178:21]
  assign Line_7_var5_valid = Line_6_var13_valid; // @[systolic_array.scala 179:21]
  assign Line_7_var5_bits = Line_6_var13_bits; // @[systolic_array.scala 179:21]
  assign Line_7_var6_valid = Line_6_var14_valid; // @[systolic_array.scala 180:21]
  assign Line_7_var6_bits = Line_6_var14_bits; // @[systolic_array.scala 180:21]
  assign Line_7_var7_valid = Line_6_var15_valid; // @[systolic_array.scala 181:21]
  assign Line_7_var7_bits = Line_6_var15_bits; // @[systolic_array.scala 181:21]
  assign Line_7_var8_valid = Line_6_var16_valid; // @[systolic_array.scala 182:21]
  assign Line_7_var8_bits = Line_6_var16_bits; // @[systolic_array.scala 182:21]
  assign Line_7_var9_ready = var33_ready; // @[systolic_array.scala 183:15]
  assign Line_7_var10_ready = var34_ready; // @[systolic_array.scala 184:15]
  assign Line_7_var11_ready = var35_ready; // @[systolic_array.scala 185:15]
  assign Line_7_var12_ready = var36_ready; // @[systolic_array.scala 186:15]
  assign Line_7_var13_ready = var37_ready; // @[systolic_array.scala 187:15]
  assign Line_7_var14_ready = var38_ready; // @[systolic_array.scala 188:15]
  assign Line_7_var15_ready = var39_ready; // @[systolic_array.scala 189:15]
  assign Line_7_var16_ready = var40_ready; // @[systolic_array.scala 190:15]
endmodule
module hec_systolic_array_8(
  input         clock,
  input         reset,
  output        var17_ready,
  input         var17_valid,
  input  [31:0] var17_bits,
  output        var18_ready,
  input         var18_valid,
  input  [31:0] var18_bits,
  output        var19_ready,
  input         var19_valid,
  input  [31:0] var19_bits,
  output        var20_ready,
  input         var20_valid,
  input  [31:0] var20_bits,
  output        var21_ready,
  input         var21_valid,
  input  [31:0] var21_bits,
  output        var22_ready,
  input         var22_valid,
  input  [31:0] var22_bits,
  output        var23_ready,
  input         var23_valid,
  input  [31:0] var23_bits,
  output        var24_ready,
  input         var24_valid,
  input  [31:0] var24_bits,
  output        var25_ready,
  input         var25_valid,
  input  [31:0] var25_bits,
  output        var26_ready,
  input         var26_valid,
  input  [31:0] var26_bits,
  output        var27_ready,
  input         var27_valid,
  input  [31:0] var27_bits,
  output        var28_ready,
  input         var28_valid,
  input  [31:0] var28_bits,
  output        var29_ready,
  input         var29_valid,
  input  [31:0] var29_bits,
  output        var30_ready,
  input         var30_valid,
  input  [31:0] var30_bits,
  output        var31_ready,
  input         var31_valid,
  input  [31:0] var31_bits,
  output        var32_ready,
  input         var32_valid,
  input  [31:0] var32_bits,
  input         var33_ready,
  output        var33_valid,
  output [31:0] var33_bits,
  input         var34_ready,
  output        var34_valid,
  output [31:0] var34_bits,
  input         var35_ready,
  output        var35_valid,
  output [31:0] var35_bits,
  input         var36_ready,
  output        var36_valid,
  output [31:0] var36_bits,
  input         var37_ready,
  output        var37_valid,
  output [31:0] var37_bits,
  input         var38_ready,
  output        var38_valid,
  output [31:0] var38_bits,
  input         var39_ready,
  output        var39_valid,
  output [31:0] var39_bits,
  input         var40_ready,
  output        var40_valid,
  output [31:0] var40_bits,
  input         finish
);
  wire  main_clock; // @[systolic_array.scala 192:26]
  wire  main_reset; // @[systolic_array.scala 192:26]
  wire  main_var17_ready; // @[systolic_array.scala 192:26]
  wire  main_var17_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var17_bits; // @[systolic_array.scala 192:26]
  wire  main_var18_ready; // @[systolic_array.scala 192:26]
  wire  main_var18_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var18_bits; // @[systolic_array.scala 192:26]
  wire  main_var19_ready; // @[systolic_array.scala 192:26]
  wire  main_var19_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var19_bits; // @[systolic_array.scala 192:26]
  wire  main_var20_ready; // @[systolic_array.scala 192:26]
  wire  main_var20_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var20_bits; // @[systolic_array.scala 192:26]
  wire  main_var21_ready; // @[systolic_array.scala 192:26]
  wire  main_var21_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var21_bits; // @[systolic_array.scala 192:26]
  wire  main_var22_ready; // @[systolic_array.scala 192:26]
  wire  main_var22_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var22_bits; // @[systolic_array.scala 192:26]
  wire  main_var23_ready; // @[systolic_array.scala 192:26]
  wire  main_var23_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var23_bits; // @[systolic_array.scala 192:26]
  wire  main_var24_ready; // @[systolic_array.scala 192:26]
  wire  main_var24_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var24_bits; // @[systolic_array.scala 192:26]
  wire  main_var25_ready; // @[systolic_array.scala 192:26]
  wire  main_var25_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var25_bits; // @[systolic_array.scala 192:26]
  wire  main_var26_ready; // @[systolic_array.scala 192:26]
  wire  main_var26_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var26_bits; // @[systolic_array.scala 192:26]
  wire  main_var27_ready; // @[systolic_array.scala 192:26]
  wire  main_var27_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var27_bits; // @[systolic_array.scala 192:26]
  wire  main_var28_ready; // @[systolic_array.scala 192:26]
  wire  main_var28_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var28_bits; // @[systolic_array.scala 192:26]
  wire  main_var29_ready; // @[systolic_array.scala 192:26]
  wire  main_var29_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var29_bits; // @[systolic_array.scala 192:26]
  wire  main_var30_ready; // @[systolic_array.scala 192:26]
  wire  main_var30_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var30_bits; // @[systolic_array.scala 192:26]
  wire  main_var31_ready; // @[systolic_array.scala 192:26]
  wire  main_var31_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var31_bits; // @[systolic_array.scala 192:26]
  wire  main_var32_ready; // @[systolic_array.scala 192:26]
  wire  main_var32_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var32_bits; // @[systolic_array.scala 192:26]
  wire  main_var33_ready; // @[systolic_array.scala 192:26]
  wire  main_var33_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var33_bits; // @[systolic_array.scala 192:26]
  wire  main_var34_ready; // @[systolic_array.scala 192:26]
  wire  main_var34_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var34_bits; // @[systolic_array.scala 192:26]
  wire  main_var35_ready; // @[systolic_array.scala 192:26]
  wire  main_var35_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var35_bits; // @[systolic_array.scala 192:26]
  wire  main_var36_ready; // @[systolic_array.scala 192:26]
  wire  main_var36_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var36_bits; // @[systolic_array.scala 192:26]
  wire  main_var37_ready; // @[systolic_array.scala 192:26]
  wire  main_var37_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var37_bits; // @[systolic_array.scala 192:26]
  wire  main_var38_ready; // @[systolic_array.scala 192:26]
  wire  main_var38_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var38_bits; // @[systolic_array.scala 192:26]
  wire  main_var39_ready; // @[systolic_array.scala 192:26]
  wire  main_var39_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var39_bits; // @[systolic_array.scala 192:26]
  wire  main_var40_ready; // @[systolic_array.scala 192:26]
  wire  main_var40_valid; // @[systolic_array.scala 192:26]
  wire [31:0] main_var40_bits; // @[systolic_array.scala 192:26]
  main_hec main_hec_comp ( // @[systolic_array.scala 192:26]
    .clock(main_clock),
    .reset(main_reset),
    .var17_ready(main_var17_ready),
    .var17_valid(main_var17_valid),
    .var17_bits(main_var17_bits),
    .var18_ready(main_var18_ready),
    .var18_valid(main_var18_valid),
    .var18_bits(main_var18_bits),
    .var19_ready(main_var19_ready),
    .var19_valid(main_var19_valid),
    .var19_bits(main_var19_bits),
    .var20_ready(main_var20_ready),
    .var20_valid(main_var20_valid),
    .var20_bits(main_var20_bits),
    .var21_ready(main_var21_ready),
    .var21_valid(main_var21_valid),
    .var21_bits(main_var21_bits),
    .var22_ready(main_var22_ready),
    .var22_valid(main_var22_valid),
    .var22_bits(main_var22_bits),
    .var23_ready(main_var23_ready),
    .var23_valid(main_var23_valid),
    .var23_bits(main_var23_bits),
    .var24_ready(main_var24_ready),
    .var24_valid(main_var24_valid),
    .var24_bits(main_var24_bits),
    .var25_ready(main_var25_ready),
    .var25_valid(main_var25_valid),
    .var25_bits(main_var25_bits),
    .var26_ready(main_var26_ready),
    .var26_valid(main_var26_valid),
    .var26_bits(main_var26_bits),
    .var27_ready(main_var27_ready),
    .var27_valid(main_var27_valid),
    .var27_bits(main_var27_bits),
    .var28_ready(main_var28_ready),
    .var28_valid(main_var28_valid),
    .var28_bits(main_var28_bits),
    .var29_ready(main_var29_ready),
    .var29_valid(main_var29_valid),
    .var29_bits(main_var29_bits),
    .var30_ready(main_var30_ready),
    .var30_valid(main_var30_valid),
    .var30_bits(main_var30_bits),
    .var31_ready(main_var31_ready),
    .var31_valid(main_var31_valid),
    .var31_bits(main_var31_bits),
    .var32_ready(main_var32_ready),
    .var32_valid(main_var32_valid),
    .var32_bits(main_var32_bits),
    .var33_ready(main_var33_ready),
    .var33_valid(main_var33_valid),
    .var33_bits(main_var33_bits),
    .var34_ready(main_var34_ready),
    .var34_valid(main_var34_valid),
    .var34_bits(main_var34_bits),
    .var35_ready(main_var35_ready),
    .var35_valid(main_var35_valid),
    .var35_bits(main_var35_bits),
    .var36_ready(main_var36_ready),
    .var36_valid(main_var36_valid),
    .var36_bits(main_var36_bits),
    .var37_ready(main_var37_ready),
    .var37_valid(main_var37_valid),
    .var37_bits(main_var37_bits),
    .var38_ready(main_var38_ready),
    .var38_valid(main_var38_valid),
    .var38_bits(main_var38_bits),
    .var39_ready(main_var39_ready),
    .var39_valid(main_var39_valid),
    .var39_bits(main_var39_bits),
    .var40_ready(main_var40_ready),
    .var40_valid(main_var40_valid),
    .var40_bits(main_var40_bits)
  );
  assign var17_ready = main_var17_ready; // @[systolic_array.scala 194:20]
  assign var18_ready = main_var18_ready; // @[systolic_array.scala 196:20]
  assign var19_ready = main_var19_ready; // @[systolic_array.scala 198:20]
  assign var20_ready = main_var20_ready; // @[systolic_array.scala 200:20]
  assign var21_ready = main_var21_ready; // @[systolic_array.scala 202:20]
  assign var22_ready = main_var22_ready; // @[systolic_array.scala 204:20]
  assign var23_ready = main_var23_ready; // @[systolic_array.scala 206:20]
  assign var24_ready = main_var24_ready; // @[systolic_array.scala 208:20]
  assign var25_ready = main_var25_ready; // @[systolic_array.scala 210:20]
  assign var26_ready = main_var26_ready; // @[systolic_array.scala 212:20]
  assign var27_ready = main_var27_ready; // @[systolic_array.scala 214:20]
  assign var28_ready = main_var28_ready; // @[systolic_array.scala 216:20]
  assign var29_ready = main_var29_ready; // @[systolic_array.scala 218:20]
  assign var30_ready = main_var30_ready; // @[systolic_array.scala 220:20]
  assign var31_ready = main_var31_ready; // @[systolic_array.scala 222:20]
  assign var32_ready = main_var32_ready; // @[systolic_array.scala 224:20]
  assign var33_valid = main_var33_valid; // @[systolic_array.scala 226:15]
  assign var33_bits = main_var33_bits; // @[systolic_array.scala 226:15]
  assign var34_valid = main_var34_valid; // @[systolic_array.scala 228:15]
  assign var34_bits = main_var34_bits; // @[systolic_array.scala 228:15]
  assign var35_valid = main_var35_valid; // @[systolic_array.scala 230:15]
  assign var35_bits = main_var35_bits; // @[systolic_array.scala 230:15]
  assign var36_valid = main_var36_valid; // @[systolic_array.scala 232:15]
  assign var36_bits = main_var36_bits; // @[systolic_array.scala 232:15]
  assign var37_valid = main_var37_valid; // @[systolic_array.scala 234:15]
  assign var37_bits = main_var37_bits; // @[systolic_array.scala 234:15]
  assign var38_valid = main_var38_valid; // @[systolic_array.scala 236:15]
  assign var38_bits = main_var38_bits; // @[systolic_array.scala 236:15]
  assign var39_valid = main_var39_valid; // @[systolic_array.scala 238:15]
  assign var39_bits = main_var39_bits; // @[systolic_array.scala 238:15]
  assign var40_valid = main_var40_valid; // @[systolic_array.scala 240:15]
  assign var40_bits = main_var40_bits; // @[systolic_array.scala 240:15]
  assign main_clock = clock;
  assign main_reset = reset;
  assign main_var17_valid = var17_valid; // @[systolic_array.scala 194:20]
  assign main_var17_bits = var17_bits; // @[systolic_array.scala 194:20]
  assign main_var18_valid = var18_valid; // @[systolic_array.scala 196:20]
  assign main_var18_bits = var18_bits; // @[systolic_array.scala 196:20]
  assign main_var19_valid = var19_valid; // @[systolic_array.scala 198:20]
  assign main_var19_bits = var19_bits; // @[systolic_array.scala 198:20]
  assign main_var20_valid = var20_valid; // @[systolic_array.scala 200:20]
  assign main_var20_bits = var20_bits; // @[systolic_array.scala 200:20]
  assign main_var21_valid = var21_valid; // @[systolic_array.scala 202:20]
  assign main_var21_bits = var21_bits; // @[systolic_array.scala 202:20]
  assign main_var22_valid = var22_valid; // @[systolic_array.scala 204:20]
  assign main_var22_bits = var22_bits; // @[systolic_array.scala 204:20]
  assign main_var23_valid = var23_valid; // @[systolic_array.scala 206:20]
  assign main_var23_bits = var23_bits; // @[systolic_array.scala 206:20]
  assign main_var24_valid = var24_valid; // @[systolic_array.scala 208:20]
  assign main_var24_bits = var24_bits; // @[systolic_array.scala 208:20]
  assign main_var25_valid = var25_valid; // @[systolic_array.scala 210:20]
  assign main_var25_bits = var25_bits; // @[systolic_array.scala 210:20]
  assign main_var26_valid = var26_valid; // @[systolic_array.scala 212:20]
  assign main_var26_bits = var26_bits; // @[systolic_array.scala 212:20]
  assign main_var27_valid = var27_valid; // @[systolic_array.scala 214:20]
  assign main_var27_bits = var27_bits; // @[systolic_array.scala 214:20]
  assign main_var28_valid = var28_valid; // @[systolic_array.scala 216:20]
  assign main_var28_bits = var28_bits; // @[systolic_array.scala 216:20]
  assign main_var29_valid = var29_valid; // @[systolic_array.scala 218:20]
  assign main_var29_bits = var29_bits; // @[systolic_array.scala 218:20]
  assign main_var30_valid = var30_valid; // @[systolic_array.scala 220:20]
  assign main_var30_bits = var30_bits; // @[systolic_array.scala 220:20]
  assign main_var31_valid = var31_valid; // @[systolic_array.scala 222:20]
  assign main_var31_bits = var31_bits; // @[systolic_array.scala 222:20]
  assign main_var32_valid = var32_valid; // @[systolic_array.scala 224:20]
  assign main_var32_bits = var32_bits; // @[systolic_array.scala 224:20]
  assign main_var33_ready = var33_ready; // @[systolic_array.scala 226:15]
  assign main_var34_ready = var34_ready; // @[systolic_array.scala 228:15]
  assign main_var35_ready = var35_ready; // @[systolic_array.scala 230:15]
  assign main_var36_ready = var36_ready; // @[systolic_array.scala 232:15]
  assign main_var37_ready = var37_ready; // @[systolic_array.scala 234:15]
  assign main_var38_ready = var38_ready; // @[systolic_array.scala 236:15]
  assign main_var39_ready = var39_ready; // @[systolic_array.scala 238:15]
  assign main_var40_ready = var40_ready; // @[systolic_array.scala 240:15]
endmodule
