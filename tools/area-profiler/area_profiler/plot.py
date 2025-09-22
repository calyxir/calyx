import argparse
import json
import plotly.express as px
from collections import defaultdict
from pathlib import Path

AREA_WEIGHTS = {
    "and": 1.0,
    "or": 1.0,
    "not": 0.5,
    "eq": 3.0,
    "logic_not": 2.0,
    "mux": 4.0,
    "std_wire": 0.2,
    "std_reg": 8.0,
}

def load_data(path: Path):
    with open(path) as f:
        return json.load(f)

def compute_areas(data):
    areas = []
    for name, cell in data.items():
        t = cell["cell_type"]
        w = cell["cell_width"]
        weight = AREA_WEIGHTS.get(t, 1.0)
        area = weight * w
        areas.append({"cell_name": name, "cell_type": t, "width": w, "area": area})
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

def main():
    parser = argparse.ArgumentParser(
        description="Estimate and plot cell areas based on a heuristic"
    )
    parser.add_argument(
        "input",
        type=Path,
        help="path to input JSON file",
    )
    parser.add_argument(
        "mode",
        choices=["bar", "treemap"],
        help="visualization type",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        help="output HTML file (default: area_by_type.html for bar, area_treemap.html for treemap)",
    )

    args = parser.parse_args()

    data = load_data(args.input)
    areas = compute_areas(data)

    if args.mode == "bar":
        output = args.output or Path("area_by_type.html")
        make_bar_chart(areas, output)
    elif args.mode == "treemap":
        output = args.output or Path("area_treemap.html")
        make_treemap(areas, output)

if __name__ == "__main__":
    main()
