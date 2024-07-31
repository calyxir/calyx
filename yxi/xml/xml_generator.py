import sys
import json
from xml.etree.ElementTree import Element, SubElement, tostring
from xml.dom import minidom
from math import log2

"""
This file takes in a `.yxi` description and outputs a xml suitable for a `kernel.xml` file
can be used to package an xclbin for the Xilinx XRT runtime.
See https://docs.amd.com/r/en-US/ug1393-vitis-application-acceleration/RTL-Kernel-XML-File
for the spec this is based on.
"""
size_key = "total_size"
width_key = "data_width"


def gen_xml(yxi):
    mems = yxi["memories"]
    check_mems_wellformed(mems)

    root = Element("root", {"versionMajor": "1", "versionMinor": "6"})
    kernel = SubElement(
        root,
        "kernel",
        {
            "name": yxi["toplevel"],
            "language": "ip_c",
            # TODO: Make sure this matches component.xml, Namely the `Toplevel` part.
            #  See https://docs.amd.com/r/en-US/ug1393-vitis-application-acceleration/RTL-Kernel-XML-File
            "vlnv": "capra.cs.cornell.edu:kernel:Toplevel:1.0",
            "attributes": "",
            "preferredWorkGroupSizeMultiple": "0",
            "workGroupSize": "1",
            "hwControlProtocol": "ap_ctrl_hs",
        },
    )

    # Construct ports
    ports = SubElement(kernel, "ports")
    # The subordinates XRT - AXI controller is added outside of the programs memory interface.
    SubElement(
        ports,
        "port",
        {
            "name": "S_AXI_CONTROL",
            "mode": "slave",
            # NOTE(nathaniel): This is 0x1000 as taken from the Xilinx examples.
            "range": "0x1000",
            "dataWidth": "32",
            "portType": "addressable",
            "base": "0x0",
        },
    )

    for mem in mems:
        SubElement(
            ports,
            "port",
            {
                "name": f"m_axi_{mem['name']}",
                "mode": "master",
                # NOTE(nathaniel): In the Xilinx examples range is usually 0xFFFFFF... but this should be fine for us?
                "range": f"{hex(size_in_bytes(mem))}",
                # NOTE(nathaniel): The old version had this hardcoded to a width of 512. This should work, but in case it doesn't we can revert to 512.
                "dataWidth": f"{mem[width_key]}",
                "portType": "addressable",
                "base": "0x0",
            },
        )

    # Construct Args
    args = SubElement(kernel, "args")
    # XRT spec starts args addresses at 0x10
    args_addr = 0x10
    for i, mem in enumerate(mems):
        SubElement(
            args,
            "arg",
            {
                "name": f"{mem['name']}",
                # 1 denotes the arguments as a global memory,
                "addressQualifier": "1",
                "id": f"{i}",
                "port": f"m_axi_{mem['name']}",
                # XRT expects AXI manager interfaces that are 64 bits wide
                "size": f"0x8",
                "offset": f"{hex(args_addr + (i * 8))}",
                # NOTE(nathaniel): Calyx is agnostic to the bit interpretation, so hardcoded `int*` makes sure XRT treats ecerything as a "bag of bits."
                # https://github.com/calyxir/calyx/pull/2229#discussion_r1694310099
                "type": "int*",
                "hostOffset": "0x0",
                "hostSize": "0x8",  # Seems to be the same as `size`, unclear how they differ
            },
        )

    return root


def size_in_bytes(mem):
    return mem[size_key] * mem[width_key] // 8


# TODO: Import from axi_generator instead of copy pasting here
def check_mems_wellformed(mems):
    """Checks if memories from yxi are well formed. Returns true if they are, false otherwise."""
    for mem in mems:
        assert (
            mem[width_key] % 8 == 0
        ), "Width must be a multiple of 8 to alow byte addressing to host"
        assert log2(
            mem[width_key]
        ).is_integer(), "Width must be a power of 2 to be correctly described by xSIZE"
        assert mem[size_key] > 0, "Memory size must be greater than 0"


def prettify(elem):
    """Return a pretty-printed XML string that is human readable.
    Mainly useful for debugging purposes.
    """
    rough_string = tostring(elem, "utf-8")
    reparsed = minidom.parseString(rough_string)
    return reparsed.toprettyxml(indent="  ")


if __name__ == "__main__":
    yxi_filename = "input.yxi"
    if len(sys.argv) != 2:
        raise Exception(
            "The `kernel.xml` generator takes 1 `.yxi` file name as an argument."
        )

    yxi_filename = sys.argv[1]
    if not yxi_filename.endswith(".yxi"):
        raise Exception("The `kernel.xml` generator requires an `.yxi` file as input.")

    with open(yxi_filename, "r", encoding="utf-8") as f:
        yxi = json.load(f)
        xml = gen_xml(yxi)
        print(tostring(xml, xml_declaration=True, encoding="unicode"))
