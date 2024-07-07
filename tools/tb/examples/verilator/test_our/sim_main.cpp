// https://veripool.org/guide/latest/example_cc.html#example-c-execution
#include "Vour.h"
#include "verilated.h"
int main(int argc, char** argv) {
    VerilatedContext* contextp = new VerilatedContext;
    contextp->commandArgs(argc, argv);
    Vour* top = new Vour{contextp};
    while (!contextp->gotFinish()) {
        top->eval();
    }
    delete top;
    delete contextp;
    return 0;
}
