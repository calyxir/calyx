from collections import Counter
import pandas as pd
import plotly.express as px
from profiler.classes import UtilizationCycleTrace


class Plotter:
    """
    Wrapper around related utilization plotting functions.
    """

    def __init__(self, data: dict[int, UtilizationCycleTrace]):
        self.data = data

    def plot_utilization_per_cycle(self, var: str, out_dir: str):
        """Plot the value of `var` per cycle."""
        records = [
            {"cycle": cycle_id, "value": obj.utilization.get(var, 0)}
            for cycle_id, obj in self.data.items()
        ]
        df = pd.DataFrame(records)
        fig = px.bar(
            df, x="cycle", y="value", title=f"Utilization of '{var}' per cycle"
        )
        fig.write_html(out_dir + "/utilization-per-cycle.html")

    def plot_cycles_per_primitive(self, var: str, out_dir: str):
        """Plot the number of cycles each primitive has a nonzero value for `var`."""
        counter = Counter()
        for obj in self.data.values():
            for prim, varmap in obj.utilization_per_primitive.items():
                if var in varmap and varmap[var] != 0:
                    counter[prim] += 1

        df = pd.DataFrame(counter.items(), columns=["primitive", "cycles_used"])
        df = df.sort_values("cycles_used", ascending=False)
        fig = px.bar(
            df,
            x="primitive",
            y="cycles_used",
            title=f"Primitives with nonzero '{var}' per cycle",
        )
        fig.write_html(out_dir + "/cycles-per-primitive.html")

    def plot_heatmap(self, var: str, out_dir: str):
        """Plot a heatmap of `var` values per primitive per cycle."""
        aggregated = []
        usage_sum = Counter()
        active_cycles = Counter()

        for cycle, trace in self.data.items():
            for prim, vars_dict in trace.utilization_per_primitive.items():
                if var in vars_dict:
                    value = int(vars_dict[var])
                    aggregated.append(
                        {"cycle": cycle, "primitive": prim, "value": value}
                    )
                    usage_sum[prim] = value
                    if value != 0:
                        active_cycles[prim] += 1

        ratios = {
            prim: (active_cycles[prim] / usage_sum[prim])
            for prim in usage_sum
            if usage_sum[prim] > 0
        }
        sorted_primitives = sorted(ratios, key=ratios.get, reverse=True)

        df = pd.DataFrame(aggregated)
        heatmap_data = df.pivot(index="primitive", columns="cycle", values="value")
        all_cycles = range(df["cycle"].min(), df["cycle"].max() + 1)
        heatmap_data = heatmap_data.reindex(columns=all_cycles)
        heatmap_data = heatmap_data.reindex(sorted_primitives)

        fig = px.imshow(
            heatmap_data,
            labels={"x": "cycle", "y": "primitive", "color": var},
            aspect="auto",
            color_continuous_scale="Bluered",
        )
        fig.update_layout(title=f"Heatmap of {var} per primitive per cycle")
        fig.write_html(out_dir + "/heatmap.html")

    def plot_ratio(self, var: str, out_dir: str):
        """Plot the ratio of active cycles to total `var` usage per primitive."""
        usage_sum = Counter()
        active_cycles = Counter()

        for obj in self.data.values():
            for prim, varmap in obj.utilization_per_primitive.items():
                if var in varmap:
                    usage_sum[prim] = int(varmap[var])
                    if varmap[var] != 0:
                        active_cycles[prim] += 1

        records = []
        for prim in usage_sum:
            area = usage_sum[prim]
            if area > 0:
                records.append(
                    {
                        "primitive": prim,
                        "ratio": active_cycles[prim] / area,
                        "area": area,
                        "cycles": active_cycles[prim],
                    }
                )

        df = pd.DataFrame(records)
        df = df.sort_values("ratio", ascending=False)

        fig = px.bar(
            df,
            x="primitive",
            y="ratio",
            hover_data=["area", "cycles"],
            title=f"Utilization-to-area ratio for '{var}' per primitive",
        )
        fig.write_html(out_dir + "/utilization-ratio.html")

    def run_all(self, var: str, out_dir: str):
        """Run all plotting methods."""
        self.plot_utilization_per_cycle(var, out_dir)
        self.plot_cycles_per_primitive(var, out_dir)
        self.plot_heatmap(var, out_dir)
        self.plot_ratio(var, out_dir)
