from pathlib import Path

class UnknownExtension(Exception):
    def __init__(self, filename):
        path = Path(filename)
        ext = path.suffix
        # TODO: suggest related suffixes
        super().__init__(f"'{ext}' doesn't correspond to a known extension.")
