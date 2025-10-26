from dataclasses import dataclass


@dataclass
class VivadoRsrc:
    """
    Vivado resources for a cell.
    """

    lut: int
    llut: int
    lutram: int
    srl: int
    ff: int
    ramb36: int
    ramb18: int
    uram: int
    dsp: int


type YosysRsrc = dict[str, int]
"""
Yosys resources for a cell.
"""


type Rsrc = VivadoRsrc | YosysRsrc
"""
Map representing resources used by a cell.
"""


@dataclass
class Cell:
    """
    Cell with resources.
    """
    
    # Unqualified cell name.
    name: str
    # Cell type.
    type: str
    # Whether the cell was generated in synthesis.
    generated: bool
    # Cell resources.
    rsrc: Rsrc

@dataclass
class Metadata:
    """
    Design metadata.
    """

    # Origin of the design (Vivado, Yosys).
    origin: str


type Design = dict[str, Cell]
"""
Design with qualified cell names and associated cells.
"""


@dataclass
class DesignWithMetadata:
    """
    Design with metadata.
    """

    design: Design
    metadata: Metadata
