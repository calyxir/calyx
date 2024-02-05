#!/bin/sh

readonly DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
readonly OUTDIR=$DIR/out

# The directory with example witnesses generated from the model checker BtorMC
readonly MCWITNESSDIR=$DIR/mc-witnesses
if [ ! -e $MCWITNESSDIR ]
then
  echo "[error] can not find MC witnesses directory"
  exit 1
fi

BTORSIM=$DIR/../../bin/btorsim
if [ ! -e $BTORSIM ]
then
  echo "[error] BtorSim not built"
  exit 1
fi

invalid () {
  :
}

mkdir -p $OUTDIR

set -x

### Check witnesses from model checker
$BTORSIM      -c     count2.btor2               $MCWITNESSDIR/count2.witnessmc
$BTORSIM      -c     count4.btor2               $MCWITNESSDIR/count4.witnessmc
$BTORSIM      -c     factorial4even.btor2       $MCWITNESSDIR/factorial4even.witnessmc
$BTORSIM      -c     noninitstate.btor2         $MCWITNESSDIR/noninitstate.witnessmc
$BTORSIM      -c     recount4.btor2             $MCWITNESSDIR/recount4.witnessmc
$BTORSIM      -c     twocount2.btor2            $MCWITNESSDIR/twocount2.witnessmc
$BTORSIM      -c     twocount2c.btor2           $MCWITNESSDIR/twocount2c.witnessmc
$BTORSIM      -c     twocount32.btor2           $MCWITNESSDIR/twocount32.witnessmc

### Simulation for sat files, simulator produces valid witnesses
# Run simulation
$BTORSIM -b 0 -r 20  count2.btor2             > $OUTDIR/count2.witnesssim
$BTORSIM -b 0 -r 20  count4.btor2             > $OUTDIR/count4.witnesssim
$BTORSIM -b 0 -r 20  factorial4even.btor2     > $OUTDIR/factorial4even.witnesssim
$BTORSIM -b 0 -r 20  noninitstate.btor2 -s 1  > $OUTDIR/noninitstate.witnesssim
$BTORSIM -b 0 -r 20  twocount2.btor2          > $OUTDIR/twocount2.witnesssim
$BTORSIM -b 0 -r 20  twocount2c.btor2   -s 11 > $OUTDIR/twocount2c.witnesssim
$BTORSIM -b 0 -r 20  twocount32.btor2         > $OUTDIR/twocount32.witnesssim
# Check witnesses produced by simulation
$BTORSIM      -c     count2.btor2               $OUTDIR/count2.witnesssim
$BTORSIM      -c     count4.btor2               $OUTDIR/count4.witnesssim
$BTORSIM      -c     factorial4even.btor2       $OUTDIR/factorial4even.witnesssim
$BTORSIM      -c     noninitstate.btor2         $OUTDIR/noninitstate.witnesssim
$BTORSIM      -c     twocount2.btor2            $OUTDIR/twocount2.witnesssim
$BTORSIM      -c     twocount2c.btor2           $OUTDIR/twocount2c.witnesssim
$BTORSIM      -c     twocount32.btor2           $OUTDIR/twocount32.witnesssim

### Simulation for sat files, simulator produces invalid witnesses
# Run simulation
$BTORSIM -b 0 -r 20  noninitstate.btor2       > $OUTDIR/noninitstate.nowitnesssim
# Check witness produced by simulation
$BTORSIM      -c     noninitstate.btor2         $OUTDIR/noninitstate.nowitnesssim || invalid
# Run simulation
$BTORSIM -b 0 -r 20  recount4.btor2           > $OUTDIR/recount4.nowitnesssim
# Check witness produced by simulation
$BTORSIM      -c     recount4.btor2             $OUTDIR/recount4.nowitnesssim     || invalid
# Run simulation
$BTORSIM -b 0 -r 20  twocount2c.btor2         > $OUTDIR/twocount2c.nowitnesssim
# Check witness produced by simulation
$BTORSIM      -c     twocount2c.btor2           $OUTDIR/twocount2c.nowitnesssim   || invalid


### Real world example
# Run simulation (invalid witness)
$BTORSIM -b 0 -r 999 ponylink-slaveTXlen-sat.btor2 > $OUTDIR/ponylink-slaveTXlen.nowitnesssim
# Check witness from simulation
$BTORSIM      -c     ponylink-slaveTXlen-sat.btor2   $OUTDIR/ponylink-slaveTXlen.nowitnesssim || invalid
# Check witness from model checker
$BTORSIM      -c     ponylink-slaveTXlen-sat.btor2   $MCWITNESSDIR/ponylink-slaveTXlen.witnessmc
