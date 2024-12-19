import sys
import calyx.builder as cb
import queues.queue_call as qc
import queues.sim_pcap as sp
import queues.strict_or_rr as st_or_rr
import queues.fifo as fifo
import queues.flow_inference as fi

NUMFLOWS = 3


if __name__ == "__main__":
    """Invoke the top-level function to build the program, with 3 flows."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    sim_pcap = "--sim-pcap" in sys.argv

    prog = cb.Builder()

    fifo_queue = fifo.insert_fifo(prog, "fifo")
    subqueues = [fifo_queue] * NUMFLOWS
    order = [1, 2, 0]
    if sim_pcap:
        flow_infer = fi.insert_tuple_flow_inference(prog, "flow_inference", NUMFLOWS)
        pifo = st_or_rr.insert_queue(
            prog, "pifo", False, subqueues, flow_infer, order=order
        )
        sp.insert_main(prog, pifo, num_cmds, NUMFLOWS)
    else:
        boundaries = [133, 266, 400]
        flow_infer = fi.insert_boundary_flow_inference(
            prog, "flow_inference", boundaries
        )
        pifo = st_or_rr.insert_queue(
            prog, "pifo", False, subqueues, flow_infer, order=order
        )
        qc.insert_main(prog, pifo, num_cmds, keepgoing=keepgoing)

    prog.program.emit()
