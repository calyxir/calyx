# Usage: python3 parse_pcap.py <PCAP> <Out> [Options]...
#
# Parses PCAP files to generate data files
#
# Positional Arguments:
#  PCAP                Packet Capture to parse
#  Out                 Path to save generated .data file
#
# Options:
#  -h --help           Display this message
#
#  --start        S    Start parsing from packet number S (0-indexed)
#                      [default: 0]
#
#  --end          E    Stop parsing at packet number E - 1 (0-indexed)
#                      [default: last packet in PCAP]
#
#  --clock-period C    Clock period of the hardware in ns
#                      [default: 7]
#
#  --line-rate    L    Target line rate for the pop frequency calculation in Gbit/s
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
#   python3 parse_pcap.py example.pcap example.data --start 10 --end 20 --num-flows 3

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
LINE_RATE = 1  # in Gbit/s
START = 0

POP_TICK = None  # in ns
ADDR2INT = None
NUM_FLOWS = None
END = None

PKTS_PER_SEC = None
BITS_PER_SEC = None


class UnknownAddress(Exception):
    pass


class OutOfBounds(Exception):
    pass


class InvalidRange(Exception):
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

    def error(s):
        raise argparse.ArgumentTypeError(s)

    def nonnegative_int(x):
        try:
            x = int(x)
            if x < 0:
                raise error(f"{x} is not a non-negative integer")
        except ValueError:
            raise error(f"{x} is not an integer")
        return x

    def positive_int(x):
        try:
            x = int(x)
            if x <= 0:
                raise error(f"{x} is not a positive integer")
        except ValueError:
            raise error(f"{x} is not an integer")
        return x

    def positive_float(x):
        try:
            x = float(x)
            if x <= 0:
                raise error(f"{x} is not a positive float")
        except ValueError:
            raise error(f"{x} is not a float")
        return x

    parser.add_argument("--start", type=nonnegative_int, action="store", default=START)
    parser.add_argument("--end", type=positive_int, action="store")
    parser.add_argument(
        "--clock-period", type=positive_int, action="store", default=CLOCK_PERIOD
    )
    parser.add_argument(
        "--pop-tick", type=positive_int, action="store", default=POP_TICK
    )
    parser.add_argument("--addr2int", action="store")
    parser.add_argument("--num-flows", type=positive_int, action="store")
    parser.add_argument(
        "--line-rate", type=positive_float, action="store", default=LINE_RATE
    )

    if "-h" in sys.argv or "--help" in sys.argv:
        parser.error()

    return parser.parse_args()


def parse_pcap(pcap_file):
    global POP_TICK
    global ADDR2INT
    global NUM_FLOWS
    global END

    global PKTS_PER_SEC
    global BITS_PER_SEC

    def mac_addr(addr):
        return ":".join("%02x" % dpkt.compat.compat_ord(b) for b in addr)

    pcap = dpkt.pcap.Reader(pcap_file)

    star_ts = None
    end_ts = None
    total_size = 0
    make_addr_map = ADDR2INT is None
    ADDR2INT = {} if ADDR2INT is None else ADDR2INT
    addr_count, pkt_count = 0, 0
    for i, (ts, buf) in enumerate(pcap):
        if i < START:
            continue
        elif i == START:
            start_ts = ts
        elif END is not None and i >= END:
            break

        eth = dpkt.ethernet.Ethernet(buf)
        addr = mac_addr(eth.src)
        if addr not in ADDR2INT:
            if make_addr_map:
                ADDR2INT[addr] = addr_count
                addr_count += 1
            else:
                raise UnknownAddress(
                    f"MAC address {addr} for packet {i} not found in Addr2Int map"
                )

        total_size += len(buf)
        pkt_count += 1
        end_ts = ts
    END = START + pkt_count if END is None else END

    if start_ts is None:
        raise OutOfBounds(f"Index {START} out of bounds for PCAP {pcap_file.name}")
    elif START >= END:
        raise InvalidRange(f"Start index {START} >= end index {END}")

    total_time = end_ts - start_ts
    PKTS_PER_SEC = float("inf") if total_time == 0 else (END - START) / total_time
    BITS_PER_SEC = float("inf") if total_time == 0 else (total_size * 8) / total_time

    if NUM_FLOWS is None:
        NUM_FLOWS = max(ADDR2INT[addr] for addr in ADDR2INT) + 1

    if POP_TICK is None:
        POP_TICK = int((total_size * 8) // (LINE_RATE * (END - START)))

    pcap_file.seek(0)
    pcap = dpkt.pcap.Reader(pcap_file)

    data = {"commands": [], "arrival_cycles": [], "flows": [], "pkt_ids": []}
    prev_time = 0
    pkts_in_switch = 0
    for i, (ts, buf) in enumerate(pcap):
        if i < START:
            continue
        elif i >= END:
            break

        time = (ts - start_ts) * 10**9

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
        flow = ADDR2INT[addr] % NUM_FLOWS
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

    return data


def dump_json(data, data_file):
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
    flows = {"flows": {"data": flows, "format": format_gen(bits_needed(NUM_FLOWS - 1))}}
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

    pcap_file = open(opts.PCAP, "rb")
    addr2int_json = None if opts.addr2int is None else open(opts.addr2int)

    ADDR2INT = None if addr2int_json is None else json.load(addr2int_json)
    CLOCK_PERIOD = opts.clock_period
    POP_TICK = opts.pop_tick
    NUM_FLOWS = opts.num_flows
    START = opts.start
    END = opts.end

    data = parse_pcap(pcap_file)

    data_file = open(opts.Out, "w")
    dump_json(data, data_file)

    stats = {}
    stats["num_cmds"] = len(data["commands"])
    stats["num_flows"] = NUM_FLOWS
    stats["addr2flow"] = {addr: ADDR2INT[addr] % NUM_FLOWS for addr in ADDR2INT}
    stats["pop_tick_ns"] = POP_TICK
    stats["pkts_per_sec"] = PKTS_PER_SEC
    stats["bits_per_sec"] = BITS_PER_SEC

    print(json.dumps(stats, indent=2))
