import re


class RPTParser:
    """
    Class implementing parsing functionality of RPT files generated by Xilinx
    tools. The core functionality is extracting tables out of these files.
    """

    SKIP_LINE = re.compile(r"^.*(\+-*)*\+$")

    def __init__(self, filepath):
        with open(filepath, "r") as data:
            self.lines = data.read().split("\n")

    @staticmethod
    def _clean_and_strip(elems):
        "Remove all empty elements from the list and strips each string element."
        nonempty = filter(lambda e: e != "\n" and e != "", elems)
        m = map(lambda e: e.strip(), nonempty)
        return list(map(lambda e: "index" if e == "" else e, m))

    @staticmethod
    def _parse_simple_header(line):
        assert re.search(
            r"\s*\|", line
        ), "Simple header line should have | as first non-whitespace character"
        return RPTParser._clean_and_strip(line.split("|"))

    @staticmethod
    def _parse_multi_header(lines):
        """
        Extract header from the form:
        +------+--------+--------+----------+-----------+-----------+
        |      |     Latency     | Iteration|  Initiation Interval  |
        | Name |   min  |   max  |  Latency |  achieved |   target  |
        +------+--------+--------+----------+-----------+-----------+

        into: ["Name", "Latency_min", "Latency_max", "Iteration Latency", ...]

        This will fail to correctly parse this header. See the comment below
        for an explanation:
        +------+--------+--------+--------+--------+
        |      |     Latency     |     Foo         |
        | Name |   min  |   max  |   bar  |   baz  |
        +------+--------+--------+--------+--------+
        turns into: ["Name", "Latency_min", "Latency_max",
                     "Latecy_bar", "Latency_baz", "Foo"]
        """

        multi_headers = []
        secondary_hdrs = lines[1].split("|")

        # Use the following heuristic to generate header names:
        # - If header starts with a small letter, it is a secondary header.
        # - If the last save sequence of headers doesn't already contain this
        #   header name, add it to the last one.
        # - Otherwise add a new sub header class.
        for idx, line in enumerate(secondary_hdrs, 1):
            clean_line = line.strip()
            if len(clean_line) == 0:
                continue
            elif (
                clean_line[0].islower()
                and len(multi_headers) > 0
                and multi_headers[-1][0].islower()
                and clean_line not in multi_headers[-1]
            ):
                multi_headers[-1].append(clean_line)
            else:
                multi_headers.append([clean_line])

        # Extract base headers and drop the starting empty lines and ending '\n'.
        base_hdrs = lines[0].split("|")[1:-1]

        if len(base_hdrs) != len(multi_headers):
            raise Exception(
                "Something went wrong while parsing multi header "
                + "base len: {}, mult len: {}".format(
                    len(base_hdrs), len(multi_headers)
                )
            )

        hdrs = []
        for idx in range(0, len(base_hdrs)):
            for mult in multi_headers[idx]:
                hdrs.append((base_hdrs[idx].strip() + " " + mult).strip())

        return hdrs

    @staticmethod
    def _parse_table(table_lines, multi_header):
        """
        Parses a simple table of the form:
        +--------+-------+----------+------------+
        |  Clock | Target| Estimated| Uncertainty|
        +--------+-------+----------+------------+
        |ap_clk  |   7.00|      4.39|        1.89|
        |ap_clk  |   7.00|      4.39|        1.89|
        +--------+-------+----------+------------+
        |ap_clk  |   7.00|      4.39|        1.89|
        +--------+-------+----------+------------+

        The might be any number of rows after the headers. The input parameter
        is a list of lines of the table starting with the top most header line.
        Return a list of dicts, one per row, whose keys come from the header
        row.

        """

        # Extract the headers and set table start
        table_start = 0
        if multi_header:
            header = RPTParser._parse_multi_header(table_lines[1:3])
            table_start = 3
        else:
            header = RPTParser._parse_simple_header(table_lines[1])
            table_start = 2

        assert len(header) > 0, "No header found"

        rows = []
        for line in table_lines[table_start:]:
            if not RPTParser.SKIP_LINE.match(line):
                rows.append(RPTParser._clean_and_strip(line.split("|")))

        ret = [
            {header[i]: row[i] for i in range(len(header))}
            for row in rows
            if len(row) == len(header)
        ]
        return ret

    def get_table(self, reg, off, multi_header=False):
        """
        Parse table `off` lines after `reg` matches the files in the current
        file.

        The table format is:
        +--------+-------+----------+------------+
        |  Clock | Target| Estimated| Uncertainty|
        +--------+-------+----------+------------+
        |ap_clk  |   7.00|      4.39|        1.89|
        |ap_clk  |   7.00|      4.39|        1.89|
        +--------+-------+----------+------------+
        |ap_clk  |   7.00|      4.39|        1.89|
        +--------+-------+----------+------------+
        """
        start = 0
        end = 0
        for idx, line in enumerate(self.lines, 1):
            if reg.search(line):
                start = idx + off

                # If start doesn't point to valid header, continue searching
                if (
                    self.lines[start].strip() == ""
                    or self.lines[start].strip()[0] != "+"
                ):
                    continue

                end = start
                while self.lines[end].strip() != "":
                    end += 1

        assert end > start, "Failed to find table start for {}.".format(reg)

        return self._parse_table(self.lines[start:end], multi_header)

    def get_bare_table(self, header_regex):
        """
        Parse a table with the format:
        ---------------------------------------------------------------------
        | Design Timing Summary
        | ---------------------
        ---------------------------------------------------------------------

            WNS(ns)      TNS(ns)  TNS Failing Endpoints  TNS Total Endpoints
            -------      -------  ---------------------  -------------------
              4.221        0.000                      0                  376

        Returns none if the table header is not found
        """

        # Iterate over the lines and find the header
        start = None
        for idx, line in enumerate(self.lines, 1):
            if header_regex.search(line):
                start = idx
                break

        if start is None:
            return None

        # Skip lines while the first non-empty word is not a letter
        while True:
            start += 1
            line = self.lines[start]
            if len(line.strip()) == 0:
                continue
            if line.strip()[0].isalpha():
                break

        # The ---- below each header defines it. First, we extract locations of
        # --- in the next line and then we extract the header from the current
        # line
        dash_line = self.lines[start + 1]
        header_line = self.lines[start]
        # Walk both the lines together
        dash_idx = 0
        headers = []

        while dash_idx < len(dash_line):
            if dash_line[dash_idx] == "-":
                # Start of a new header
                cur_header = ""
                while dash_idx < len(dash_line) and dash_line[dash_idx] == "-":
                    cur_header += header_line[dash_idx]
                    dash_idx += 1
                headers.append(cur_header.strip())
            else:
                # If we've found a non-dash, skip it
                while dash_idx < len(dash_line) and dash_line[dash_idx] != "-":
                    dash_idx += 1

        # The next line is the separator. Skip it
        start += 2
        # The final line is the data. Split up the line and remove empty strings
        data = list(filter(lambda a: a != "", self.lines[start].split(" ")))

        # Return a dict with the headers as keys and the data as values
        return {headers[i]: data[i] for i in range(len(headers))}
