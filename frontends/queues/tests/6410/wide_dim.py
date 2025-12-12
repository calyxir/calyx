# pylint: disable=import-error
import sys
import random
import calyx.builder as cb
import queues.queue_call as qc
import queues.strict_or_rr as strict_or_rr
import queues.fifo as fifo
import queues.flow_inference as fi

# -----------------------------
# CONFIGURATION
# -----------------------------
BRANCH = 8  # branching factor per internal node
MAX_VALUE = 400  # max value to insert
SEED = 12345  # for deterministic randomness


# -----------------------------
# HELPER FUNCTIONS
# -----------------------------
def make_random_fi(prog, name_prefix, num_children):
    """
    Create a boundary flow inference node with random priorities.
    """
    priorities = [random.randint(100, 1000) for _ in range(num_children)]
    return fi.insert_boundary_flow_inference(prog, f"{name_prefix}_fi", priorities)


def make_random_queue(prog, name_prefix, children, fi_node, len_factor=4):
    """
    Randomly choose SP or RR for an internal queue.
    By default, queues have length 2^len_factor = 16.
    """
    is_rr = random.choice([True, False])
    if is_rr:
        return strict_or_rr.insert_queue(
            prog, f"{name_prefix}_rr", True, children, fi_node, len_factor
        )
    else:
        priorities = list(range(len(children)))
        return strict_or_rr.insert_queue(
            prog, f"{name_prefix}_sp", False, children, fi_node, priorities, len_factor
        )


def route_value(value):
    """
    Map value 1..MAX_VALUE to (mid_index, leaf_index)
    """
    if value < 1:
        value = 1
    if value > MAX_VALUE:
        value = MAX_VALUE

    # There are BRANCH * BRANCH = 64 leaves
    bucket_size = MAX_VALUE // (BRANCH * BRANCH)  # integer division
    bucket = min((value - 1) // bucket_size, BRANCH * BRANCH - 1)
    mid_index = bucket // BRANCH
    leaf_index = bucket % BRANCH
    return mid_index, leaf_index


# -----------------------------
# MAIN BUILD FUNCTION
# -----------------------------
def build():
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv

    random.seed(SEED)

    prog = cb.Builder()

    # -----------------------------
    # 1. CREATE 8x8 FIFOS AND MID-LEVEL QUEUES
    # -----------------------------
    fifo_leaves = []  # 8x8 list to store leaf FIFOs
    mid_level_nodes = []

    for i in range(BRANCH):
        leaf_fifos = []
        for j in range(BRANCH):
            leaf = fifo.insert_fifo(prog, f"fifo_{i}_{j}", 1)  # FIFO of length 2
            leaf_fifos.append(leaf)
        fifo_leaves.append(leaf_fifos)

        fi_node = make_random_fi(prog, f"mid_{i}", BRANCH)
        qnode = make_random_queue(prog, f"mid_{i}", leaf_fifos, fi_node)
        mid_level_nodes.append(qnode)

    # -----------------------------
    # 2. CREATE ROOT QUEUE
    # -----------------------------
    fi_root = make_random_fi(prog, "root", BRANCH)
    root = make_random_queue(
        prog, "root", mid_level_nodes, fi_root, 7
    )  # a queue of length 2^7 = 128

    # -----------------------------
    # 3. PREPARE COMMANDS TO ROUTE VALUES
    # -----------------------------
    commands = []
    for val in range(1, num_cmds + 1):
        mid_i, leaf_j = route_value(val)
        target_fifo = fifo_leaves[mid_i][leaf_j]
        commands.append((target_fifo, val))

    # -----------------------------
    # 4. INSERT MAIN
    # -----------------------------
    qc.insert_main(prog, root, num_cmds, keepgoing=keepgoing)

    return prog.program


# -----------------------------
# ENTRY POINT
# -----------------------------
if __name__ == "__main__":
    build().emit()
