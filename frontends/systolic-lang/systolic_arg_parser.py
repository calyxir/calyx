import argparse
import json

SUPPORTED_POST_OPS = ["leaky-relu"]


class SystolicConfiguration:
    """
    A class that represents a "systolic configuration". Includes:
    Top length, top depth, left length, left depth.
    Post Op (default=None)
    """

    def parse_arguments(self):
        """
        Parses arguments and returns the following outputs:
        top_length, top_depth, left_length, left_depth, leaky_relu
        """
        import argparse
        import json

        # Arg parsing
        parser = argparse.ArgumentParser(description="Process some integers.")
        parser.add_argument("file", nargs="?", type=str)
        parser.add_argument("-tl", "--top-length", type=int)
        parser.add_argument("-td", "--top-depth", type=int)
        parser.add_argument("-ll", "--left-length", type=int)
        parser.add_argument("-ld", "--left-depth", type=int)
        parser.add_argument("-p", "--post-op", type=str, required=False)

        args = parser.parse_args()

        fields = [args.top_length, args.top_depth, args.left_length, args.left_depth]
        if all(map(lambda x: x is not None, fields)):
            self.top_length = args.top_length
            self.top_depth = args.top_depth
            self.left_length = args.left_length
            self.left_depth = args.left_depth
            self.post_op = args.post_op
        elif args.file is not None:
            with open(args.file, "r") as f:
                spec = json.load(f)
                self.top_length = spec["top_length"]
                self.top_depth = spec["top_depth"]
                self.left_length = spec["left_length"]
                self.left_depth = spec["left_depth"]
                # default to not perform leaky_relu
                self.post_op = spec.get("post_op", False)
        else:
            parser.error(
                "Need to pass either `FILE` or all of `"
                "-tl TOP_LENGTH -td TOP_DEPTH -ll LEFT_LENGTH -ld LEFT_DEPTH`"
            )
        assert self.top_depth == self.left_depth, (
            f"Cannot multiply matrices: "
            f"{self.top_length}x{self.top_depth} and {self.left_depth}x{self.left_length}"
        )
