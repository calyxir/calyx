import xml.etree.ElementTree as ET
import sys


def get_ports(kernel_xml):
    tree = ET.parse(kernel_xml)
    for port in tree.findall(".//port[@mode='master']"):
        yield port.attrib["name"]


if __name__ == "__main__":
    print(' '.join(get_ports(sys.argv[1])))
