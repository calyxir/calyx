# pylint: disable=import-error
import pifo
import builder_util as util
import calyx.builder as cb


def insert_pifo_tree(prog):
    """A PIFO tree that achieves a 50/50 split between two flows, and
    further 50/50 split between its first flow.

    This is achieved by maintaining three PIFOs:
    - `pifo_0`: a PIFO that contains indices 1 or 2.
    - `pifo_1`: a PIFO that contains values from flow 1, i.e. 0-100.
      it split flow 1 further into two flows, flow 3 (0-50) and flow 4 (51-100),
      and gives them equal priority.
    - `pifo_2`: a PIFO that contains values from flow 2, i.e. 101-200.


    - len(pifo_tree) = len(pifo_0)
    - `push(v, f, pifotree)`:
       + If len(pifotree) = 10, raise an "overflow" err and exit.
       + Otherwise, the charge is to enqueue value `v`, that is known to be from
         flow `f`, and `f` better be `2`, `3`, or `4`.
         Enqueue `v` into `pifo_1` if `f` is `3` or `4`, and into `pifo_2` otherwise.
         Note that the PIFO's enqueue method is itself partial: it may raise
         "overflow", in which case we propagate the overflow flag.
    - `pop(pifotree)`:
       + If `len(pifotree)` = 0, raise an "underflow" flag and exit.
       + Perform pop(pifo_0). It will return an index `i` that is either 1 or 2.
         Perform pop(pifo_i). It will return a value `v`. Propagate `v`.
    """
