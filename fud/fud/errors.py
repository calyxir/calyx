from pathlib import Path


class UnknownExtension(Exception):
    def __init__(self, filename):
        path = Path(filename)
        ext = path.suffix
        # TODO: suggest related suffixes
        super().__init__(f"'{ext}' doesn't correspond to a known extension.")


class MissingDynamicConfiguration(Exception):
    def __init__(self, variable):
        msg = f"'{variable}' needs to be set. " + \
            "Try again with: '-s {variable} <value>'."
        super().__init__(msg)


class NoPathFound(Exception):
    def __init__(self, source, destination):
        msg = f"No path found from {source} to {destination}."
        super().__init__(msg)
