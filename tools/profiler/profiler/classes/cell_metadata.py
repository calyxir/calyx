from dataclasses import dataclass, field
from profiler.classes.errors import ProfilerException


@dataclass
class CellMetadata:
    """
    Preprocessed information related to cells.
    """

    main_component: str
    # component name --> [cell names]
    component_to_cells: dict[str, list[str]]
    # component name --> { old cell --> new cell}
    shared_cells: dict[str, dict[str, str]] = field(default_factory=dict)
    added_signal_prefix: bool = field(default=False)

    # optional fields to fill in later

    # OS-specific Verilator prefix
    signal_prefix: str | None = field(default=None)

    def add_signal_prefix(self, signal_prefix: str):
        """
        Add OS-specific Verilator prefix to all cell names
        """
        assert not self.added_signal_prefix
        self.signal_prefix = signal_prefix
        str_to_add = signal_prefix + "."
        self.main_component = str_to_add + self.main_component

        for component in self.component_to_cells:
            fq_cells = [
                f"{signal_prefix}.{cell}" for cell in self.component_to_cells[component]
            ]
            self.component_to_cells[component] = fq_cells

        self.added_signal_prefix = True

    def get_component_of_cell(self, cell: str):
        """
        Obtain the name of the component from which a cell comes from.
        """
        for component in self.component_to_cells:
            if cell in self.component_to_cells[component]:
                return component
        raise ProfilerException(
            f"Lookup of cell that doesn't have a corresponding component! Cell name: {cell}"
        )

    @property
    def cells(self) -> list[str]:
        cells = []
        for component in self.component_to_cells:
            cells += self.component_to_cells[component]
        return cells

    @property
    def main_shortname(self):
        # Name of the main component without the signal prefix
        suffix = self.main_component.rsplit(".", 1)[-1]
        return suffix
