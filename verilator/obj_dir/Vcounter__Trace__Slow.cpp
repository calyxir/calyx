// Verilated -*- C++ -*-
// DESCRIPTION: Verilator output: Tracing implementation internals
#include "verilated_vcd_c.h"
#include "Vcounter__Syms.h"


//======================

void Vcounter::trace(VerilatedVcdC* tfp, int, int) {
    tfp->spTrace()->addCallback(&Vcounter::traceInit, &Vcounter::traceFull, &Vcounter::traceChg, this);
}
void Vcounter::traceInit(VerilatedVcd* vcdp, void* userthis, uint32_t code) {
    // Callback from vcd->open()
    Vcounter* t = (Vcounter*)userthis;
    Vcounter__Syms* __restrict vlSymsp = t->__VlSymsp;  // Setup global symbol table
    if (!Verilated::calcUnusedSigs()) {
        VL_FATAL_MT(__FILE__, __LINE__, __FILE__,
                        "Turning on wave traces requires Verilated::traceEverOn(true) call before time 0.");
    }
    vcdp->scopeEscape(' ');
    t->traceInitThis(vlSymsp, vcdp, code);
    vcdp->scopeEscape('.');
}
void Vcounter::traceFull(VerilatedVcd* vcdp, void* userthis, uint32_t code) {
    // Callback from vcd->dump()
    Vcounter* t = (Vcounter*)userthis;
    Vcounter__Syms* __restrict vlSymsp = t->__VlSymsp;  // Setup global symbol table
    t->traceFullThis(vlSymsp, vcdp, code);
}

//======================


void Vcounter::traceInitThis(Vcounter__Syms* __restrict vlSymsp, VerilatedVcd* vcdp, uint32_t code) {
    Vcounter* __restrict vlTOPp VL_ATTR_UNUSED = vlSymsp->TOPp;
    int c = code;
    if (0 && vcdp && c) {}  // Prevent unused
    vcdp->module(vlSymsp->name());  // Setup signal names
    // Body
    {
        vlTOPp->traceInitThis__1(vlSymsp, vcdp, code);
    }
}

void Vcounter::traceFullThis(Vcounter__Syms* __restrict vlSymsp, VerilatedVcd* vcdp, uint32_t code) {
    Vcounter* __restrict vlTOPp VL_ATTR_UNUSED = vlSymsp->TOPp;
    int c = code;
    if (0 && vcdp && c) {}  // Prevent unused
    // Body
    {
        vlTOPp->traceFullThis__1(vlSymsp, vcdp, code);
    }
    // Final
    vlTOPp->__Vm_traceActivity = 0U;
}

void Vcounter::traceInitThis__1(Vcounter__Syms* __restrict vlSymsp, VerilatedVcd* vcdp, uint32_t code) {
    Vcounter* __restrict vlTOPp VL_ATTR_UNUSED = vlSymsp->TOPp;
    int c = code;
    if (0 && vcdp && c) {}  // Prevent unused
    // Body
    {
        vcdp->declBit(c+1,"clk",-1);
        vcdp->declBit(c+2,"rst",-1);
        vcdp->declBit(c+3,"cen",-1);
        vcdp->declBit(c+4,"wen",-1);
        vcdp->declBus(c+5,"dat",-1,7,0);
        vcdp->declBus(c+6,"o_p",-1,7,0);
        vcdp->declBus(c+7,"o_n",-1,7,0);
        vcdp->declBus(c+8,"counter WIDTH",-1,31,0);
        vcdp->declBit(c+1,"counter clk",-1);
        vcdp->declBit(c+2,"counter rst",-1);
        vcdp->declBit(c+3,"counter cen",-1);
        vcdp->declBit(c+4,"counter wen",-1);
        vcdp->declBus(c+5,"counter dat",-1,7,0);
        vcdp->declBus(c+6,"counter o_p",-1,7,0);
        vcdp->declBus(c+7,"counter o_n",-1,7,0);
    }
}

void Vcounter::traceFullThis__1(Vcounter__Syms* __restrict vlSymsp, VerilatedVcd* vcdp, uint32_t code) {
    Vcounter* __restrict vlTOPp VL_ATTR_UNUSED = vlSymsp->TOPp;
    int c = code;
    if (0 && vcdp && c) {}  // Prevent unused
    // Body
    {
        vcdp->fullBit(c+1,(vlTOPp->clk));
        vcdp->fullBit(c+2,(vlTOPp->rst));
        vcdp->fullBit(c+3,(vlTOPp->cen));
        vcdp->fullBit(c+4,(vlTOPp->wen));
        vcdp->fullBus(c+5,(vlTOPp->dat),8);
        vcdp->fullBus(c+6,(vlTOPp->o_p),8);
        vcdp->fullBus(c+7,(vlTOPp->o_n),8);
        vcdp->fullBus(c+8,(8U),32);
    }
}
