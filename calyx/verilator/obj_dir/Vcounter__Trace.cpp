// Verilated -*- C++ -*-
// DESCRIPTION: Verilator output: Tracing implementation internals
#include "verilated_vcd_c.h"
#include "Vcounter__Syms.h"


//======================

void Vcounter::traceChg(VerilatedVcd* vcdp, void* userthis, uint32_t code) {
    // Callback from vcd->dump()
    Vcounter* t = (Vcounter*)userthis;
    Vcounter__Syms* __restrict vlSymsp = t->__VlSymsp;  // Setup global symbol table
    if (vlSymsp->getClearActivity()) {
        t->traceChgThis(vlSymsp, vcdp, code);
    }
}

//======================


void Vcounter::traceChgThis(Vcounter__Syms* __restrict vlSymsp, VerilatedVcd* vcdp, uint32_t code) {
    Vcounter* __restrict vlTOPp VL_ATTR_UNUSED = vlSymsp->TOPp;
    int c = code;
    if (0 && vcdp && c) {}  // Prevent unused
    // Body
    {
        vlTOPp->traceChgThis__2(vlSymsp, vcdp, code);
    }
    // Final
    vlTOPp->__Vm_traceActivity = 0U;
}

void Vcounter::traceChgThis__2(Vcounter__Syms* __restrict vlSymsp, VerilatedVcd* vcdp, uint32_t code) {
    Vcounter* __restrict vlTOPp VL_ATTR_UNUSED = vlSymsp->TOPp;
    int c = code;
    if (0 && vcdp && c) {}  // Prevent unused
    // Body
    {
        vcdp->chgBit(c+1,(vlTOPp->clk));
        vcdp->chgBit(c+2,(vlTOPp->rst));
        vcdp->chgBit(c+3,(vlTOPp->cen));
        vcdp->chgBit(c+4,(vlTOPp->wen));
        vcdp->chgBus(c+5,(vlTOPp->dat),8);
        vcdp->chgBus(c+6,(vlTOPp->o_p),8);
        vcdp->chgBus(c+7,(vlTOPp->o_n),8);
    }
}
