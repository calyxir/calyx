import sys
import calyx.builder as cb
import queues.queue_call as qc
import queues.binheap.round_robin as rr
import queues.flow_inference as fi 

NUMFLOWS = 6


if __name__ == "__main__":
    """Invoke the top-level function to build the program, with 6 flows."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    sim_pcap = "--sim-pcap" in sys.argv

    prog = cb.Builder()

    if sim_pcap:
        raise Exception("Not Implemented")
    else:
        boundaries = [66, 100, 200, 220, 300, 400]
        flow_infer = fi.insert_boundary_flow_inference(prog, "flow_inference", boundaries)
        pifo = rr.insert_binheap_rr(prog, "pifo", NUMFLOWS, flow_infer)
        qc.insert_main(prog, pifo, num_cmds, keepgoing=keepgoing)

    prog.program.emit()
