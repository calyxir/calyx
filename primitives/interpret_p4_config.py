from dataclasses import dataclass
from uuid import uuid4
from typing import List
from io import TextIOWrapper
from calyx.py_ast import *


@dataclass
class Action:
    table_name: str
    action_name: str
    ip: str
    subnet_mask: str
    action_data: List[str]

    action_address: int
    action_id: int

    def __init__(
        self,
        table_name: str,
        action_name: str,
        ip: str,
        subnet_mask: str,
        action_data: str,
        action_address: int,
    ):
        self.table_name = table_name
        self.action_name = action_name
        self.ip = ip
        self.subnet_mask = subnet_mask
        self.action_data = action_data
        self.action_address = action_address

        # Generate a random, unique address for simplicity.
        # Realistically, this would be an ID that could be
        # interpreted by the Action ALU.
        addr = uuid4().hex[:8]
        self.action_id = int(addr, 16)


def interpret_p4_config(file: TextIOWrapper, unique_address: int) -> List[Action]:
    """Given a P4 Switch Config, returns a list of Actions.
    Each action is given a unique address in the Action Memory.
    Currently, an action ID is stripped from UUID4. Eventually,
    this should be an op code for the Action ALU."""

    def parse_action(line: str, action_address: int) -> Action:
        """Interprets a P4 configuration table line of the form:
        table_add <table_name> <action> <match>/<length> => <action data(s)>"""
        tokens = line.strip().split(" ")
        assert (
            tokens[0] == "table_add" and tokens[4] == "=>"
        ), f"""The configuration line: {line} 
does not match the expected form:
table_add <table_name> <action> <match>/<length> => <action data(s)>"""

        ip, subnet_mask = tokens[3].split("/", maxsplit=1)
        return Action(
            table_name=tokens[1],
            action_name=tokens[2],
            ip=ip,
            subnet_mask=subnet_mask,
            action_data=[] if len(tokens) <= 5 else tokens[5:],
            action_address=action_address,
        )

    actions = []
    for line in file.readlines():
        actions.append(parse_action(line, unique_address))
        unique_address = unique_address + 1

    return actions


def write_to_action_memory(action_memory_id: CompVar, actions: List[Action]):
    """Writes each action ID to an address in the Action Memory."""
    groups = []
    for a in actions:
        action_addr = a.action_address
        action_id = a.action_id

        # Write `action_id` to `action_addr`.
        group_name = CompVar(f"wr_action_memory_{action_id}")
        connections = [
            Connect(
                ConstantPort(32, action_id), CompPort(action_memory_id, "write_data")
            ),
            Connect(ConstantPort(5, action_addr), CompPort(action_memory_id, "addr0")),
            Connect(ConstantPort(1, 1), CompPort(action_memory_id, "write_en")),
            Connect(CompPort(action_memory_id, "done"), HolePort(group_name, "done")),
        ]
        groups.append(Group(group_name, connections, static_delay=1))

    return groups, SeqComp([Enable(g.id.name) for g in groups])


def ip_to_decimal(ip: str) -> int:
    octet1 = 16777216  # 256 ** 3
    octet2 = 65536  # 256 ** 2
    octet3 = 256
    ips = ip.split(".", maxsplit=3)
    ips = [int(ip) for ip in ips]

    return ips[0] * octet1 + ips[1] * octet2 + ips[2] * octet3 + ips[3]


def write_to_match_engine(match_engine_id: CompVar, actions: List[Action]):
    """Writes each IP (with prefix length) to the Match Engine."""
    controls = []
    for a in actions:
        # Write mapping (ip, mask) -> address.
        address = a.action_address
        ip = a.ip
        prefix_length = int(a.subnet_mask)

        # Right shift the prefix since we want the
        # masking bits to be on the RHS.
        prefix = ip_to_decimal(ip) >> (32 - prefix_length)

        controls.append(
            Invoke(
                id=match_engine_id,
                in_connects=[
                    ("write_en", ConstantPort(1, 1)),
                    ("write_index", ConstantPort(5, address)),
                    ("in", ConstantPort(32, prefix, representation="binary")),
                    ("prefix_len", ConstantPort(6, prefix_length)),
                ],
                out_connects=[],
            )
        )

    return SeqComp(controls)


if __name__ == "__main__":
    import argparse, json

    parser = argparse.ArgumentParser(description="Interpret P4 Table Configuration")
    parser.add_argument("file", nargs="?", type=str)
    args = parser.parse_args()

    # A counter to ensure each action receives a unique address in the Action Memory.
    unique_address = 0

    if args.file is not None:
        with open(args.file, "r") as file:
            actions = interpret_p4_config(file, unique_address)
    else:
        parser.error("Need to pass in `-f FILE`.")

    action_memory_id = CompVar("action_memory")
    match_engine_id = CompVar("tcam")
    groups, control0 = write_to_action_memory(action_memory_id, actions)
    control1 = write_to_match_engine(match_engine_id, actions)

    main = Component(
        name="main",
        inputs=[],
        outputs=[],
        structs=[
            Cell(action_memory_id, Stdlib().mem_d1(32, 32, 5)),
            Cell(match_engine_id, CompInst("TCAM_IPv4", [])),
        ]
        + groups,
        controls=ParComp([control0, control1]),
    )

    Program(
        imports=[Import("primitives/core.futil"), Import("primitives/tcam.futil")],
        components=[main],
    ).emit()
