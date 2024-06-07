import calyx.builder as cb

def insert_main_component(prog):
  """
  This creates the main component which does all the matrix 
  multiplication work.
  """
  comp = prog.component("main")
  mem1 = comp.comb_mem_d2("mem1", 32, 2, 2, 1, 1, is_external=True)
  mem2 = comp.comb_mem_d2("mem2", 32, 2, 2, 1, 1, is_external=True)
  mem3 = comp.comb_mem_d2("mem3", 32, 2, 2, 1, 1, is_external=True)
  val = comp.reg(32)
  val1 = comp.reg(32)
  val2 = comp.reg(32)
  val3 = comp.reg(32)
  val4 = comp.reg(32)
  val5 = comp.reg(32)
  val6 = comp.reg(32)
  val7 = comp.reg(32)
  val8 = comp.reg(32)
  temp1 = comp.reg(32)
  temp2 = comp.reg(32)

  add = comp.add(32)
  mul = comp.mult_pipe(32) #is it just mul?

  with comp.group("read1") as read1:
    mem1.addr0 = cb.LO
    mem1.addr1 = cb.LO
    val1.in_ = mem1.read_data
    val1.write_en = cb.HI
    read1.done = val1.done

  with comp.group("read2") as read2:
    mem2.addr0 = cb.LO
    mem2.addr1 = cb.LO
    val2.in_ = mem2.read_data
    val2.write_en = cb.HI
    read2.done = val1.done

  with comp.group("read3") as read3:
    mem1.addr0 = cb.LO
    mem1.addr1 = cb.HI
    val3.in_ = mem1.read_data
    val3.write_en = cb.HI
    read3.done = val3.done

  with comp.group("read4") as read4:
    mem2.addr0 = cb.LO
    mem2.addr1 = cb.HI
    val4.in_ = mem2.read_data
    val4.write_en = cb.HI
    read4.done = val4.done

  with comp.group("read5") as read5:
    mem1.addr0 = cb.HI
    mem1.addr1 = cb.LO
    val5.in_ = mem1.read_data
    val5.write_en = cb.HI
    read5.done = val5.done

  with comp.group("read6") as read6:
    mem2.addr0 = cb.HI
    mem2.addr1 = cb.LO
    val6.in_ = mem2.read_data
    val6.write_en = cb.HI
    read6.done = val6.done

  with comp.group("read7") as read7:
    mem1.addr0 = cb.HI
    mem1.addr1 = cb.HI
    val7.in_ = mem1.read_data
    val7.write_en = cb.HI
    read7.done = val7.done

  with comp.group("read8") as read8:
    mem2.addr0 = cb.HI
    mem2.addr1 = cb.HI
    val8.in_ = mem2.read_data
    val8.write_en = cb.HI
    read8.done = val8.done

  with comp.group("mul_upd") as mul_upd:
    mul.left = val1.out
    mul.right = val2.out
    mul.go = cb.HI
    temp1.in_ = mul.out
    temp1.write_en = mul.done
    mul_upd.done = temp1.done

  with comp.group("mul_upd2") as mul_upd2:
    mul.left = val3.out
    mul.right = val6.out
    mul.go = cb.HI
    temp2.in_ = mul.out
    temp2.write_en = mul.done
    mul_upd.done = temp2.done

  with comp.group("mul_upd3") as mul_upd3:
    mul.left = val1.out
    mul.right = val4.out
    mul.go = cb.HI
    temp1.in_ = mul.out
    temp1.write_en = mul.done
    mul_upd3.done = temp1.done

  with comp.group("mul_upd4") as mul_upd4:
    mul.left = val3.out
    mul.right = val8.out
    mul.go = cb.HI
    temp2.in_ = mul.out
    temp2.write_en = mul.done
    mul_upd4.done = temp2.done

  with comp.group("mul_upd5") as mul_upd5:
    mul.left = val5.out
    mul.right = val2.out
    mul.go = cb.HI
    temp1.in_ = mul.out
    temp1.write_en = mul.done
    mul_upd5.done = temp1.done

  with comp.group("mul_upd6") as mul_upd6:
    mul.left = val7.out
    mul.right = val6.out
    mul.go = cb.HI
    temp2.in_ = mul.out
    temp2.write_en = mul.done
    mul_upd6.done = temp2.done

  with comp.group("mul_upd7") as mul_upd7:
    mul.left = val5.out
    mul.right = val4.out
    mul.go = cb.HI
    temp1.in_ = mul.out
    temp1.write_en = mul.done
    mul_upd7.done = temp1.done

  with comp.group("mul_upd8") as mul_upd8:
    mul.left = val7.out
    mul.right = val8.out
    mul.go = cb.HI
    temp2.in_ = mul.out
    temp2.write_en = mul.done
    mul_upd8.done = temp2.done

  upd, _ = comp.add_store_in_reg(temp1.out, temp2.out, val)

  with comp.group("write") as write:
    mem3.addr0 = cb.LO
    mem3.addr1 = cb.LO
    mem3.write_en = cb.HI
    mem3.write_data = val.out
    write.done = mem3.done

  with comp.group("write1") as write1:
    mem3.addr0 = cb.HI
    mem3.addr1 = cb.LO
    mem3.write_en = cb.HI
    mem3.write_data = val.out
    write1.done = mem3.done

  with comp.group("write2") as write2:
    mem3.addr0 = cb.LO
    mem3.addr1 = cb.HI
    mem3.write_en = cb.HI
    mem3.write_data = val.out
    write2.done = mem3.done

  with comp.group("write3") as write3:
    mem3.addr0 = cb.HI
    mem3.addr1 = cb.HI
    mem3.write_en = cb.HI
    mem3.write_data = val.out
    write3.done = mem3.done

  comp.control += [
    cb.par(read1, read2),
    cb.par(read3, read4),
    cb.par(read5, read6),
    cb.par(read7, read8),
    mul_upd,
    mul_upd2,
    upd,
    write,
    mul_upd3,
    mul_upd4,
    upd,
    write2,
    mul_upd5,
    mul_upd6,
    upd,
    write1,
    mul_upd7,
    mul_upd8,
    upd,
    write3
  ]

if __name__ == "__main__":
    prog = cb.Builder()
    prog.import_("primitives/memories/comb.futil")
    prog.import_("primitives/binary_operators.futil")
    insert_main_component(prog)
    prog.program.emit()

    