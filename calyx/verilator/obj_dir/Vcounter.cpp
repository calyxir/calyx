// Verilated -*- C++ -*-
// DESCRIPTION: Verilator output: Design implementation internals
// See Vcounter.h for the primary calling header

#include "Vcounter.h"
#include "Vcounter__Syms.h"


//--------------------
// STATIC VARIABLES


//--------------------

VL_CTOR_IMP(Vcounter) {
    Vcounter__Syms* __restrict vlSymsp = __VlSymsp = new Vcounter__Syms(this, name());
    Vcounter* __restrict vlTOPp VL_ATTR_UNUSED = vlSymsp->TOPp;
    // Reset internal values
    
    // Reset structure values
    _ctor_var_reset();
}

void Vcounter::__Vconfigure(Vcounter__Syms* vlSymsp, bool first) {
    if (0 && first) {}  // Prevent unused
    this->__VlSymsp = vlSymsp;
}

Vcounter::~Vcounter() {
    delete __VlSymsp; __VlSymsp=NULL;
}

//--------------------


void Vcounter::eval() {
    VL_DEBUG_IF(VL_DBG_MSGF("+++++TOP Evaluate Vcounter::eval\n"); );
    Vcounter__Syms* __restrict vlSymsp = this->__VlSymsp;  // Setup global symbol table
    Vcounter* __restrict vlTOPp VL_ATTR_UNUSED = vlSymsp->TOPp;
#ifdef VL_DEBUG
    // Debug assertions
    _eval_debug_assertions();
#endif  // VL_DEBUG
    // Initialize
    if (VL_UNLIKELY(!vlSymsp->__Vm_didInit)) _eval_initial_loop(vlSymsp);
    // Evaluate till stable
    int __VclockLoop = 0;
    QData __Vchange = 1;
    do {
        VL_DEBUG_IF(VL_DBG_MSGF("+ Clock loop\n"););
        vlSymsp->__Vm_activity = true;
        _eval(vlSymsp);
        if (VL_UNLIKELY(++__VclockLoop > 100)) {
            // About to fail, so enable debug to see what's not settling.
            // Note you must run make with OPT=-DVL_DEBUG for debug prints.
            int __Vsaved_debug = Verilated::debug();
            Verilated::debug(1);
            __Vchange = _change_request(vlSymsp);
            Verilated::debug(__Vsaved_debug);
            VL_FATAL_MT("counter.v", 1, "",
                "Verilated model didn't converge\n"
                "- See DIDNOTCONVERGE in the Verilator manual");
        } else {
            __Vchange = _change_request(vlSymsp);
        }
    } while (VL_UNLIKELY(__Vchange));
}

void Vcounter::_eval_initial_loop(Vcounter__Syms* __restrict vlSymsp) {
    vlSymsp->__Vm_didInit = true;
    _eval_initial(vlSymsp);
    vlSymsp->__Vm_activity = true;
    // Evaluate till stable
    int __VclockLoop = 0;
    QData __Vchange = 1;
    do {
        _eval_settle(vlSymsp);
        _eval(vlSymsp);
        if (VL_UNLIKELY(++__VclockLoop > 100)) {
            // About to fail, so enable debug to see what's not settling.
            // Note you must run make with OPT=-DVL_DEBUG for debug prints.
            int __Vsaved_debug = Verilated::debug();
            Verilated::debug(1);
            __Vchange = _change_request(vlSymsp);
            Verilated::debug(__Vsaved_debug);
            VL_FATAL_MT("counter.v", 1, "",
                "Verilated model didn't DC converge\n"
                "- See DIDNOTCONVERGE in the Verilator manual");
        } else {
            __Vchange = _change_request(vlSymsp);
        }
    } while (VL_UNLIKELY(__Vchange));
}

//--------------------
// Internal Methods

VL_INLINE_OPT void Vcounter::_sequent__TOP__1(Vcounter__Syms* __restrict vlSymsp) {
    VL_DEBUG_IF(VL_DBG_MSGF("+    Vcounter::_sequent__TOP__1\n"); );
    Vcounter* __restrict vlTOPp VL_ATTR_UNUSED = vlSymsp->TOPp;
    // Body
    vlTOPp->o_p = ((IData)(vlTOPp->rst) ? 0U : (0xffU 
                                                & ((IData)(vlTOPp->wen)
                                                    ? (IData)(vlTOPp->dat)
                                                    : 
                                                   ((IData)(vlTOPp->o_p) 
                                                    + (IData)(vlTOPp->cen)))));
}

VL_INLINE_OPT void Vcounter::_sequent__TOP__2(Vcounter__Syms* __restrict vlSymsp) {
    VL_DEBUG_IF(VL_DBG_MSGF("+    Vcounter::_sequent__TOP__2\n"); );
    Vcounter* __restrict vlTOPp VL_ATTR_UNUSED = vlSymsp->TOPp;
    // Body
    vlTOPp->o_n = ((IData)(vlTOPp->rst) ? 0U : (0xffU 
                                                & ((IData)(vlTOPp->wen)
                                                    ? (IData)(vlTOPp->dat)
                                                    : 
                                                   ((IData)(vlTOPp->o_n) 
                                                    + (IData)(vlTOPp->cen)))));
}

void Vcounter::_eval(Vcounter__Syms* __restrict vlSymsp) {
    VL_DEBUG_IF(VL_DBG_MSGF("+    Vcounter::_eval\n"); );
    Vcounter* __restrict vlTOPp VL_ATTR_UNUSED = vlSymsp->TOPp;
    // Body
    if ((((IData)(vlTOPp->clk) & (~ (IData)(vlTOPp->__Vclklast__TOP__clk))) 
         | ((IData)(vlTOPp->rst) & (~ (IData)(vlTOPp->__Vclklast__TOP__rst))))) {
        vlTOPp->_sequent__TOP__1(vlSymsp);
    }
    if ((((~ (IData)(vlTOPp->clk)) & (IData)(vlTOPp->__Vclklast__TOP__clk)) 
         | ((IData)(vlTOPp->rst) & (~ (IData)(vlTOPp->__Vclklast__TOP__rst))))) {
        vlTOPp->_sequent__TOP__2(vlSymsp);
    }
    // Final
    vlTOPp->__Vclklast__TOP__clk = vlTOPp->clk;
    vlTOPp->__Vclklast__TOP__rst = vlTOPp->rst;
}

void Vcounter::_eval_initial(Vcounter__Syms* __restrict vlSymsp) {
    VL_DEBUG_IF(VL_DBG_MSGF("+    Vcounter::_eval_initial\n"); );
    Vcounter* __restrict vlTOPp VL_ATTR_UNUSED = vlSymsp->TOPp;
    // Body
    vlTOPp->__Vclklast__TOP__clk = vlTOPp->clk;
    vlTOPp->__Vclklast__TOP__rst = vlTOPp->rst;
}

void Vcounter::final() {
    VL_DEBUG_IF(VL_DBG_MSGF("+    Vcounter::final\n"); );
    // Variables
    Vcounter__Syms* __restrict vlSymsp = this->__VlSymsp;
    Vcounter* __restrict vlTOPp VL_ATTR_UNUSED = vlSymsp->TOPp;
}

void Vcounter::_eval_settle(Vcounter__Syms* __restrict vlSymsp) {
    VL_DEBUG_IF(VL_DBG_MSGF("+    Vcounter::_eval_settle\n"); );
    Vcounter* __restrict vlTOPp VL_ATTR_UNUSED = vlSymsp->TOPp;
}

VL_INLINE_OPT QData Vcounter::_change_request(Vcounter__Syms* __restrict vlSymsp) {
    VL_DEBUG_IF(VL_DBG_MSGF("+    Vcounter::_change_request\n"); );
    Vcounter* __restrict vlTOPp VL_ATTR_UNUSED = vlSymsp->TOPp;
    // Body
    // Change detection
    QData __req = false;  // Logically a bool
    return __req;
}

#ifdef VL_DEBUG
void Vcounter::_eval_debug_assertions() {
    VL_DEBUG_IF(VL_DBG_MSGF("+    Vcounter::_eval_debug_assertions\n"); );
    // Body
    if (VL_UNLIKELY((clk & 0xfeU))) {
        Verilated::overWidthError("clk");}
    if (VL_UNLIKELY((rst & 0xfeU))) {
        Verilated::overWidthError("rst");}
    if (VL_UNLIKELY((cen & 0xfeU))) {
        Verilated::overWidthError("cen");}
    if (VL_UNLIKELY((wen & 0xfeU))) {
        Verilated::overWidthError("wen");}
}
#endif // VL_DEBUG

void Vcounter::_ctor_var_reset() {
    VL_DEBUG_IF(VL_DBG_MSGF("+    Vcounter::_ctor_var_reset\n"); );
    // Body
    clk = VL_RAND_RESET_I(1);
    rst = VL_RAND_RESET_I(1);
    cen = VL_RAND_RESET_I(1);
    wen = VL_RAND_RESET_I(1);
    dat = VL_RAND_RESET_I(8);
    o_p = VL_RAND_RESET_I(8);
    o_n = VL_RAND_RESET_I(8);
    __Vm_traceActivity = 0;
}
