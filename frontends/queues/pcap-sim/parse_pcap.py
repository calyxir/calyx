# =============================================================================
# Usage: python3 parse_pcap.py <PCAP> <Addr2Flow> <Out> [Option]...
# =============================================================================
# Arguments:
#  PCAP                Packet Capture to parse
#  Addr2Flow           JSON mapping MAC addresses to integer flows
#  Out                 Path to save generated .data file
#
# Options:
#  -h --help           Display this message
#
#  --num-packets  N    No. packets in PCAP to parse
#                      [default: 1000]
#
#  --clock-period C    Clock period of hardware in ns
#                      [default: 7]
#
#  --line-rate    L    Target line rate for pop frequency calculation in Gbit/s
#                      [default: 1]
#
#  --pop-tick     P    Time between consecutive pops in ns
#                      [default: calulated to achieve line rate]
#
#  --num-flows    F    No. flows
#                      The flow of a packet is Addr2Flow[Packet Address] mod F
#                      [default: max value in Addr2Flow + 1]
#
# Example:
#   python3 parse_pcap.py example.pcap addr2flow.json example.data --num-packets 500

import sys
import random
import json
import dpkt
import argparse
from calyx.utils import bits_needed

CMD_PUSH = 1
CMD_POP = 0
DONTCARE = 0

CLOCK_PERIOD = 7  # in ns
NUM_PKTS = 500
POP_TICK = None  # in ns
LINE_RATE = 1  # in Gbit/s


class ArgumentParserWithCustomError(argparse.ArgumentParser):
    def __init__(self):
        super().__init__(add_help=False)

    def error(self, msg=None):
        if msg:
            print("ERROR: %s" % msg)
        file = open(sys.argv[0])
        for i, line in enumerate(file):
            if line[0] == "#":
                print(line[2:].rstrip("\n"))
            else:
                sys.exit(1 if msg else 0)


def parse_cmdline():
    parser = ArgumentParserWithCustomError()

    parser.add_argument("-h", "--help", action="store_true")
    parser.add_argument("PCAP")
    parser.add_argument("Addr2Flow")
    parser.add_argument("Out")

    def check_positive_int(x):
        try:
            x = int(x)
            if x <= 0:
                raise argparse.ArgumentTypeError(f"{x} is not a positive integer")
        except ValueError:
            raise argparse.ArgumentTypeError(f"{x} is not an integer")
        return x

    parser.add_argument(
        "--num-packets", type=check_positive_int, action="store", default=NUM_PKTS
    )
    parser.add_argument(
        "--clock-period", type=check_positive_int, action="store", default=CLOCK_PERIOD
    )
    parser.add_argument(
        "--pop-tick", type=check_positive_int, action="store", default=POP_TICK
    )
    parser.add_argument("--num-flows", type=check_positive_int, action="store")

    def check_positive_float(x):
        try:
            x = float(x)
            if x <= 0:
                raise argparse.ArgumentTypeError(f"{x} is not a positive float")
        except ValueError:
            raise argparse.ArgumentTypeError(f"{x} is not a float")
        return x

    parser.add_argument(
        "--line-rate", type=check_positive_float, action="store", default=LINE_RATE
    )

    if "-h" in sys.argv or "--help" in sys.argv:
        parser.error()

    return parser.parse_args()


def parse_pcap(pcap, addr2flow, num_flows):
    global POP_TICK

    offset = None
    total_size = 0
    for i, (ts, buf) in zip(range(NUM_PKTS), pcap):
        if i == 0:
            offset = ts
        total_size += len(buf)

    if POP_TICK is None:
        POP_TICK = int((total_size * 8) // (LINE_RATE * NUM_PKTS))

    def mac_addr(addr):
        return ":".join("%02x" % dpkt.compat.compat_ord(b) for b in addr)

    pcap_file.seek(0)
    pcap = dpkt.pcap.Reader(pcap_file)
    out = {"commands": [], "arrival_cycles": [], "flows": [], "pkt_ids": []}
    prev_time = 0
    pkts_in_switch = 0
    for i, (ts, buf) in zip(range(NUM_PKTS), pcap):
        time = (ts - offset) * 10**9

        pop_time = (prev_time % POP_TICK) + prev_time
        num_pops = int((time - pop_time) // POP_TICK) if time > pop_time else 0
        pkts_in_switch = 0 if pkts_in_switch < num_pops else pkts_in_switch - num_pops
        for _ in range(num_pops):
            out["commands"].append(CMD_POP)

            pop_cycle = int(pop_time // CLOCK_PERIOD)
            out["arrival_cycles"].append(pop_cycle)
            pop_time += POP_TICK

            out["flows"].append(DONTCARE)
            out["pkt_ids"].append(DONTCARE)

        eth = dpkt.ethernet.Ethernet(buf)
        flow = addr2flow[mac_addr(eth.src)] % num_flows
        cycle = int(time // CLOCK_PERIOD)
        pkts_in_switch += 1

        out["commands"].append(CMD_PUSH)
        out["arrival_cycles"].append(cycle)
        out["flows"].append(flow)
        out["pkt_ids"].append(i)

        prev_time = time

    pop_time = (prev_time % POP_TICK) + prev_time
    for _ in range(pkts_in_switch):
        out["commands"].append(CMD_POP)

        pop_cycle = int(pop_time // CLOCK_PERIOD)
        out["arrival_cycles"].append(pop_cycle)
        pop_time += POP_TICK

        out["flows"].append(DONTCARE)
        out["pkt_ids"].append(DONTCARE)

    return out


def dump_json(data, flow_bits, data_file):
    commands = data["commands"]
    arrival_cycles = data["arrival_cycles"]
    flows = data["flows"]
    values = data["pkt_ids"]
    ans_mem = [0] * len(commands)
    departure_cycles = [0] * len(commands)

    def format_gen(width):
        return {"is_signed": False, "numeric_type": "bitnum", "width": width}

    commands = {"commands": {"data": commands, "format": format_gen(1)}}
    arrival_cycles = {
        "arrival_cycles": {"data": arrival_cycles, "format": format_gen(32)}
    }
    flows = {"flows": {"data": flows, "format": format_gen(flow_bits)}}
    values = {"values": {"data": values, "format": format_gen(32)}}
    ans_mem = {"ans_mem": {"data": ans_mem, "format": format_gen(32)}}
    departure_cycles = {
        "departure_cycles": {"data": departure_cycles, "format": format_gen(32)}
    }

    json.dump(
        commands | values | arrival_cycles | flows | ans_mem | departure_cycles,
        data_file,
        indent=2,
    )


if __name__ == "__main__":
    opts = parse_cmdline()

    CLOCK_PERIOD = opts.clock_period
    NUM_PKTS = opts.num_packets
    POP_TICK = opts.pop_tick

    with open(opts.PCAP, "rb") as pcap_file:
        with open(opts.Addr2Flow) as addr2flow_json:
            pcap = dpkt.pcap.Reader(pcap_file)
            addr2flow = json.load(addr2flow_json)
            if opts.num_flows is None:
                num_flows = max(addr2flow[addr] for addr in addr2flow) + 1
            else:
                num_flows = opts.num_flows
            data = parse_pcap(pcap, addr2flow, num_flows)

            num_cmds = len(data["commands"])
            print(f'len(data["commands"] = {num_cmds}')

            with open(opts.Out, "w") as data_file:
                flow_bits = bits_needed(num_flows - 1)
                json = dump_json(data, flow_bits, data_file)
