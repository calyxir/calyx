# Debugging Calyx
This is a document that accumulates tips about how to go about debugging Calyx programs.

## Incorrect behavior
Viewing the value of signals at every clock cycle is a good way to make sure that written
Calyx program is doing what you expect. Use:
```
fud e buggy.futil -o buggy.vcd -s verilog.data 'buggy.data'
```
to compile your program into the Value Change Dump (VCD) format. Once you have this file,
use a wave viewer program like [GTKWave][gtkwave] or [WaveTrace][wavetrace] to look at the
wave form.

Knowing where to look in the wave form itself can be overwhelming. My favorite method for
locating something in a waveform is using the `-p compile` flag to generate the control
fsms in the control program, but leave all the groups in tact so that it's easy to find
the logical place in the program you want to debug. Consider this `dot-product.futil` taken
from `examples/futil/dot-product.futil`:
```
import "primitives/std.lib";
component main() -> () {
  cells {
    @external(1) A0 = std_mem_d1(32,8,4);
    A_read0_0 = std_reg(32);
    @external(1) B0 = std_mem_d1(32,8,4);
    B_read0_0 = std_reg(32);
    add0 = std_add(32);
    add1 = std_add(4);
    bin_read0_0 = std_reg(32);
    const0 = std_const(4,0);
    const1 = std_const(4,7);
    const2 = std_const(1,0);
    const3 = std_const(1,0);
    const4 = std_const(4,1);
    dot_0 = std_reg(32);
    i0 = std_reg(4);
    le0 = std_le(4);
    mult_pipe0 = std_mult_pipe(32);
    @external(1) v0 = std_mem_d1(32,1,1);
  }
  wires {
    group cond0<"static"=0> {
      cond0[done] = 1'd1;
      le0.left = i0.out;
      le0.right = const1.out;
    }
    group let0<"static"=1> {
      i0.in = const0.out;
      i0.write_en = 1'd1;
      let0[done] = i0.done;
    }
    group let1<"static"=4> {
      bin_read0_0.in = mult_pipe0.out;
      bin_read0_0.write_en = mult_pipe0.done;
      let1[done] = bin_read0_0.done;
      mult_pipe0.left = A_read0_0.out;
      mult_pipe0.right = B_read0_0.out;
      mult_pipe0.go = !mult_pipe0.done ? 1'd1;
    }
    group let2<"static"=1> {
      dot_0.in = bin_read0_0.out;
      dot_0.write_en = 1'd1;
      let2[done] = dot_0.done;
    }
    group upd0<"static"=1> {
      A_read0_0.write_en = 1'd1;
      A0.addr0 = i0.out;
      A_read0_0.in = 1'd1 ? A0.read_data;
      upd0[done] = A_read0_0.done ? 1'd1;
    }
    group upd1<"static"=1> {
      B_read0_0.write_en = 1'd1;
      B0.addr0 = i0.out;
      B_read0_0.in = 1'd1 ? B0.read_data;
      upd1[done] = B_read0_0.done ? 1'd1;
    }
    group upd2<"static"=1> {
      v0.addr0 = const3.out;
      v0.write_en = 1'd1;
      add0.left = v0.read_data;
      add0.right = dot_0.out;
      v0.addr0 = const2.out;
      v0.write_data = 1'd1 ? add0.out;
      upd2[done] = v0.done ? 1'd1;
    }
    group upd3<"static"=1> {
      i0.write_en = 1'd1;
      add1.left = i0.out;
      add1.right = const4.out;
      i0.in = 1'd1 ? add1.out;
      upd3[done] = i0.done ? 1'd1;
    }
  }
  control {
    seq {
      let0;
      while le0.out with cond0 {
        seq {
          par {
            upd0;
            upd1;
          }
          let1;
          let2;
          upd2;
          upd3;
        }
      }
    }
  }
}
```
Suppose that we want to make sure that the group `upd0` is correctly
reading the value in `A0[i]`. Compiling this program with `futil dot-product.futil -p compile`
yields:
```
import "primitives/std.lib";
component main(go: 1, clk: 1) -> (done: 1) {
  cells {
    @external(1) A0 = std_mem_d1(32, 8, 4);
    A_read0_0 = std_reg(32);
    @external(1) B0 = std_mem_d1(32, 8, 4);
    B_read0_0 = std_reg(32);
    add0 = std_add(32);
    add1 = std_add(4);
    bin_read0_0 = std_reg(32);
    const0 = std_const(4, 0);
    const1 = std_const(4, 7);
    const2 = std_const(1, 0);
    const3 = std_const(1, 0);
    const4 = std_const(4, 1);
    dot_0 = std_reg(32);
    i0 = std_reg(4);
    le0 = std_le(4);
    mult_pipe0 = std_mult_pipe(32);
    @external(1) v0 = std_mem_d1(32, 1, 1);
    fsm = std_reg(1);
    incr = std_add(1);
    fsm0 = std_reg(4);
    incr0 = std_add(4);
    fsm1 = std_reg(4);
    cond_stored = std_reg(1);
    incr1 = std_add(4);
    fsm2 = std_reg(2);
  }
  wires {
    group cond0<"static"=0> {
      cond0[done] = 1'd1;
      le0.left = i0.out;
      le0.right = const1.out;
    }
    group let0<"static"=1> {
      i0.in = const0.out;
      i0.write_en = 1'd1;
      let0[done] = i0.done;
    }
    group let1<"static"=4> {
      bin_read0_0.in = mult_pipe0.out;
      bin_read0_0.write_en = mult_pipe0.done;
      let1[done] = bin_read0_0.done;
      mult_pipe0.left = A_read0_0.out;
      mult_pipe0.right = B_read0_0.out;
      mult_pipe0.go = !mult_pipe0.done ? 1'd1;
    }
    group let2<"static"=1> {
      dot_0.in = bin_read0_0.out;
      dot_0.write_en = 1'd1;
      let2[done] = dot_0.done;
    }
    group upd0<"static"=1> {
      A_read0_0.write_en = 1'd1;
      A0.addr0 = i0.out;
      A_read0_0.in = A0.read_data;
      upd0[done] = A_read0_0.done ? 1'd1;
    }
    group upd1<"static"=1> {
      B_read0_0.write_en = 1'd1;
      B0.addr0 = i0.out;
      B_read0_0.in = B0.read_data;
      upd1[done] = B_read0_0.done ? 1'd1;
    }
    group upd2<"static"=1> {
      v0.addr0 = const3.out;
      v0.write_en = 1'd1;
      add0.left = v0.read_data;
      add0.right = dot_0.out;
      v0.addr0 = const2.out;
      v0.write_data = add0.out;
      upd2[done] = v0.done ? 1'd1;
    }
    group upd3<"static"=1> {
      i0.write_en = 1'd1;
      add1.left = i0.out;
      add1.right = const4.out;
      i0.in = add1.out;
      upd3[done] = i0.done ? 1'd1;
    }
    group static_par<"static"=1> {
      incr.left = 1'd1;
      incr.right = fsm.out;
      fsm.in = fsm.out != 1'd1 ? incr.out;
      fsm.write_en = fsm.out != 1'd1 ? 1'd1;
      static_par[done] = fsm.out == 1'd1 ? 1'd1;
      upd0[go] = fsm.out < 1'd1 ? 1'd1;
      upd1[go] = fsm.out < 1'd1 ? 1'd1;
    }
    group static_seq<"static"=8> {
      static_par[go] = fsm0.out == 4'd0 ? 1'd1;
      let1[go] = fsm0.out >= 4'd1 & fsm0.out < 4'd5 ? 1'd1;
      let2[go] = fsm0.out == 4'd5 ? 1'd1;
      upd2[go] = fsm0.out == 4'd6 ? 1'd1;
      upd3[go] = fsm0.out == 4'd7 ? 1'd1;
      incr0.left = 4'd1;
      incr0.right = fsm0.out;
      fsm0.in = fsm0.out != 4'd8 ? incr0.out;
      fsm0.write_en = fsm0.out != 4'd8 ? 1'd1;
      static_seq[done] = fsm0.out == 4'd8 ? 1'd1;
    }
    group static_while {
      incr1.left = fsm1.out;
      incr1.right = 4'd1;
      fsm1.in = fsm1.out != 4'd9 ? incr1.out;
      fsm1.write_en = fsm1.out != 4'd9 ? 1'd1;
      cond0[go] = fsm1.out < 4'd1 ? 1'd1;
      cond_stored.write_en = fsm1.out < 4'd1 ? 1'd1;
      static_seq[go] = cond_stored.out & fsm1.out >= 4'd1 & fsm1.out < 4'd9 ? 1'd1;
      fsm1.in = fsm1.out == 4'd9 ? 4'd0;
      fsm1.write_en = fsm1.out == 4'd9 ? 1'd1;
      static_while[done] = fsm1.out == 4'd1 & !cond_stored.out ? 1'd1;
      cond_stored.in = fsm1.out < 4'd1 ? le0.out;
    }
    group tdcc {
      let0[go] = !let0[done] & fsm2.out == 2'd0 ? 1'd1;
      static_while[go] = !static_while[done] & fsm2.out == 2'd1 ? 1'd1;
      fsm2.in = fsm2.out == 2'd0 & let0[done] ? 2'd1;
      fsm2.write_en = fsm2.out == 2'd0 & let0[done] ? 1'd1;
      fsm2.in = fsm2.out == 2'd1 & static_while[done] ? 2'd2;
      fsm2.write_en = fsm2.out == 2'd1 & static_while[done] ? 1'd1;
      tdcc[done] = fsm2.out == 2'd2 ? 1'd1;
    }
    fsm.in = fsm.out == 1'd1 ? 1'd0;
    fsm.write_en = fsm.out == 1'd1 ? 1'd1;
    fsm0.in = fsm0.out == 4'd8 ? 4'd0;
    fsm0.write_en = fsm0.out == 4'd8 ? 1'd1;
    fsm1.in = fsm1.out == 4'd1 & !cond_stored.out ? 4'd0;
    fsm1.write_en = fsm1.out == 4'd1 & !cond_stored.out ? 1'd1;
    fsm2.in = fsm2.out == 2'd2 ? 2'd0;
    fsm2.write_en = fsm2.out == 2'd2 ? 1'd1;
  }

  control {
    tdcc;
  }
}
```
The first thing to do is locate the generated group that controls the group `upd0`.
Look for a group that assigns to `upd0[go]`. This is the group that controls when
`upd0` should start. In this case, `static_par` is the group that starts `upd0`:
```
    ...
    group static_par<"static"=1> {
      incr.left = 1'd1;
      incr.right = fsm.out;
      fsm.in = fsm.out != 1'd1 ? incr.out;
      fsm.write_en = fsm.out != 1'd1 ? 1'd1;
      static_par[done] = fsm.out == 1'd1 ? 1'd1;
      upd0[go] = fsm.out < 1'd1 ? 1'd1;    <---- starts upd0
      upd1[go] = fsm.out < 1'd1 ? 1'd1;
    }
    ...
```
From the assignment to `upd0[go]`, we can find the fsm register that controls when `upd0` should run.
In this case the register is called `fsm` and `upd0` is enabled when `fsm.out < 1`.
You can now open the vcd file, look at the `fsm.out` signal, and use this to orient yourself
when looking at other signals.

[gtkwave]: http://gtkwave.sourceforge.net/
[wavetrace]: https://marketplace.visualstudio.com/items?itemName=wavetrace.wavetrace
