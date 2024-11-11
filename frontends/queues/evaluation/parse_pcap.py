# Usage: python3 parse_pcap.py <PCAP> <Out> [Options]...
#
# Parses PCAP files to generate data files
#
# Positional Arguments:
#  PCAP                Packet Capture to parse
#  Addr2Flow           JSON mapping MAC addresses to integer flows
#  Out                 Path to save generated .data file
#
# Options:
#  -h --help           Display this message
#
#  --num-packets  N    No. packets in PCAP to parse
#                      [default: 500]
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
#  --addr2int     A    JSON mapping MAC addresses to non-negative integers
#                      [default: ith encountered address -> i]
#
#  --num-flows    F    No. flows
#                      The flow of a packet is Addr2Int[Packet Address] mod F
#                      [default: max value in Addr2Int + 1]
#
# Example:
#   python3 parse_pcap.py example.pcap example.data --addr2int addr2int.json --num-packets 250

import sys
import random
import json
import dpkt
import argparse
from contextlib import nullcontext
from calyx.utils import bits_needed

CMD_PUSH = 1
CMD_POP = 0
DONTCARE = 0

CLOCK_PERIOD = 7  # in ns
NUM_PKTS = 500
POP_TICK = None  # in ns
LINE_RATE = 1  # in Gbit/s


class UnknownAddress(Exception):
    pass


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
    parser.add_argument("--addr2int", action="store")
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


def parse_pcap(pcap, addr2int, num_flows):
    global POP_TICK

    def mac_addr(addr):
        return ":".join("%02x" % dpkt.compat.compat_ord(b) for b in addr)

    offset = None
    total_size = 0
    make_addr_map = addr2int == None
    if make_addr_map:
        addr2int = {}
        addr_count = 0
    for i, (ts, buf) in zip(range(NUM_PKTS), pcap):
        if i == 0:
            offset = ts

        eth = dpkt.ethernet.Ethernet(buf)
        addr = mac_addr(eth.src)
        if addr not in addr2int:
            if make_addr_map:
                addr2int[addr] = addr_count
                addr_count += 1
            else:
                raise UnknownAddress(
                    f"MAC address {addr} for packet {i} not found in Addr2Flow map:\n {addr2int}"
                )

        total_size += len(buf)

    if num_flows is None:
        num_flows = max(addr2int[addr] for addr in addr2int) + 1

    if POP_TICK is None:
        POP_TICK = int((total_size * 8) // (LINE_RATE * NUM_PKTS))

    pcap_file.seek(0)
    pcap = dpkt.pcap.Reader(pcap_file)
    data = {"commands": [], "arrival_cycles": [], "flows": [], "pkt_ids": []}
    prev_time = 0
    pkts_in_switch = 0
    for i, (ts, buf) in zip(range(NUM_PKTS), pcap):
        time = (ts - offset) * 10**9

        pop_time = (prev_time % POP_TICK) + prev_time
        num_pops = int((time - pop_time) // POP_TICK) if time > pop_time else 0
        pkts_in_switch = 0 if pkts_in_switch < num_pops else pkts_in_switch - num_pops
        for _ in range(num_pops):
            data["commands"].append(CMD_POP)

            pop_cycle = int(pop_time // CLOCK_PERIOD)
            data["arrival_cycles"].append(pop_cycle)
            pop_time += POP_TICK

            data["flows"].append(DONTCARE)
            data["pkt_ids"].append(DONTCARE)

        eth = dpkt.ethernet.Ethernet(buf)
        addr = mac_addr(eth.src)
        flow = addr2int[addr] % num_flows
        cycle = int(time // CLOCK_PERIOD)
        pkt_id = i + 1
        pkts_in_switch += 1

        data["commands"].append(CMD_PUSH)
        data["arrival_cycles"].append(cycle)
        data["flows"].append(flow)
        data["pkt_ids"].append(pkt_id)

        prev_time = time

    pop_time = (prev_time % POP_TICK) + prev_time
    for _ in range(pkts_in_switch):
        data["commands"].append(CMD_POP)

        pop_cycle = int(pop_time // CLOCK_PERIOD)
        data["arrival_cycles"].append(pop_cycle)
        pop_time += POP_TICK

        data["flows"].append(DONTCARE)
        data["pkt_ids"].append(DONTCARE)

    return data, num_flows, addr2int


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
        with (
            nullcontext() if opts.addr2int is None else open(opts.addr2int)
        ) as addr2int_json:
            pcap = dpkt.pcap.Reader(pcap_file)
            addr2int = None if addr2int_json is None else json.load(addr2int_json)
            num_flows = opts.num_flows

            data, num_flows, addr2int = parse_pcap(pcap, addr2int, num_flows)

            with open(opts.Out, "w") as data_file:
                flow_bits = bits_needed(num_flows - 1)
                json = dump_json(data, flow_bits, data_file)

            print(f"Number of commands = {len(data['commands'])}")
            print("Addresses to flows:")
            for addr in addr2int:
                print(f"\t{addr} -> {addr2int[addr] % num_flows}")
            print(f"Pop tick = {POP_TICK} ns or {POP_TICK / CLOCK_PERIOD} cycles")
