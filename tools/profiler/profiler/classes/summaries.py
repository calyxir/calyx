from dataclasses import dataclass, field
import statistics


@dataclass
class GroupSummary:
    """
    Summary for groups on the number of times they were active vs their active cycles
    """

    display_name: str
    num_times_active: int = 0
    active_cycles: set[int] = field(default_factory=set)

    interval_lengths: list[int] = field(default_factory=list)

    def register_interval(self, interval: range):
        self.num_times_active += 1
        self.active_cycles.update(set(interval))
        self.interval_lengths.append(len(interval))

    def fieldnames():
        return [
            "group-name",
            "num-times-active",
            "total-cycles",
            "min",
            "max",
            "avg",
            "can-static",
        ]

    def stats(self):
        stats = {}
        stats["group-name"] = self.display_name
        stats["num-times-active"] = self.num_times_active
        stats["total-cycles"] = len(self.active_cycles)
        min_interval = min(self.interval_lengths)
        max_interval = max(self.interval_lengths)
        avg_interval = round(statistics.mean(self.interval_lengths), 1)
        stats["min"] = min_interval
        stats["max"] = max_interval
        stats["avg"] = avg_interval
        stats["can-static"] = "Y" if min_interval == max_interval else "N"
        return stats


@dataclass
class Summary:
    """
    Summary for Cells/Control groups on the number of times they were active vs their active cycles
    FIXME: Add min/max/avg and collect these for normal groups as well?
    """

    num_times_active: int = 0
    active_cycles: set[int] = field(default_factory=set)
