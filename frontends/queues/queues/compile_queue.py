# pylint: disable=import-error
import sys
import calyx.builder as cb
import queues.queue_call as qc
import queues.strict_or_rr as strict_or_rr
import queues.fifo as fifo
import queues.flow_inference as fi
import json
import os

rr_id = 0
strict_id = 0


def create(data, lower, upper, prog, fifo_queue):
    """
    Recursively creates the PIFO by crawling the json file `data`, which represents
    a Rio program. `lower' and `upper` are the bounds for which a flow is restricted
    to.
    """
    global rr_id
    global strict_id
    if isinstance(data, dict):
        for key, val in data.items():
            if key == "FIFO":
                return fifo_queue
            elif key == "RR" or key == "Strict":
                num_children = len(val)
                if num_children == 1:
                    return create(val[0], lower, upper, prog, fifo_queue)
                else:
                    interval = (upper - lower) // num_children
                    boundaries = []
                    for i in range(1, num_children):
                        boundaries.append(lower + (interval * i))
                    boundaries.append(upper)

                    children = []
                    lo = lower
                    u = upper
                    for i in range(num_children):
                        u = lo + interval
                        if i == num_children - 1:
                            u = upper
                        children.append(create(val[i], lo, u, prog, fifo_queue))
                        lo = u

                    if key == "RR":
                        rr_id += 1
                        return strict_or_rr.insert_queue(
                            prog,
                            f"pifo_rr{rr_id}",
                            True,
                            children,
                            fi.insert_boundary_flow_inference(
                                prog, f"fi_rr{rr_id}", boundaries
                            ),
                        )
                    elif key == "Strict":
                        strict_id += 1
                        return strict_or_rr.insert_queue(
                            prog,
                            f"pifo_strict{strict_id}",
                            False,
                            children,
                            fi.insert_boundary_flow_inference(
                                prog, f"fi_strict{strict_id}", boundaries
                            ),
                            order=[n for n in range(num_children)],
                        )


def build(json_file):
    """
    Top-level function to build the program. Requires `json_file` to be in the same
    directory as this program.
    """
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()

    base_dir = os.path.dirname(__file__)
    json_subdir = "../tests/compiler/jsons"
    file_path = os.path.join(base_dir, json_subdir, json_file)
    with open(file_path) as f:
        data = json.load(f)

    fifo_queue = fifo.insert_fifo(prog, "fifo")
    root = create(data, 0, 400, prog, fifo_queue)

    qc.insert_main(prog, root, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
