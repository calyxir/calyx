import json
import plotly.express as px
from collections import defaultdict
from pathlib import Path
from os import mkdir
from AreaExtract.lib.cdf.cdf import (
    DesignWithMetadata,
)


def load_data(path: Path):
    with open(path) as f:
        return json.load(f)


def compute_areas(design: DesignWithMetadata, column):
    areas = []
    if not column:
        column = "width" if design.metadata.origin == "Yosys" else "ff"
    for name, cell in design.design.items():
        t = cell.type
        d = cell.rsrc[column]
        areas.append({"cell_name": name, "cell_type": t, "area": d})
    return areas


def make_bar_chart(areas, output):
    type_area = defaultdict(float)
    for a in areas:
        type_area[a["cell_type"]] += a["area"]
    summary = [{"cell_type": t, "total_area": area} for t, area in type_area.items()]

    fig = px.bar(
        summary,
        x="cell_type",
        y="total_area",
        title="estimated area",
        labels={"total_area": "Estimated area"},
    )
    fig.write_html(output)


def make_treemap(areas, output):
    fig = px.treemap(
        areas,
        path=["cell_type", "cell_name"],
        values="area",
        title="estimated area treemap",
    )
    fig.write_html(output)


def save_plot(design: DesignWithMetadata, output: Path, column: str):
    output = output or "aext-out"
    mkdir(Path(output))
    bar_out = Path(output, "area_by_type.html")
    tree_out = Path(output, "area_treemap.html")

    areas = compute_areas(design, column)

    make_bar_chart(areas, bar_out)
    make_treemap(areas, tree_out)
