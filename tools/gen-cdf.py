import json
import sys
import matplotlib.pyplot as plt

if __name__ == "__main__":
    """
    Takes in a pdf json and a cell type, and plots the corresponding cdf graph.
    Usually, you will generate the json by passing `-x cell-share:print-share-freqs=file-name`
    as a flag.
    """
    assert (len(sys.argv) == 3), "please provide an input json file name and a cell_type"
    json_file = sys.argv[1]
    cell_type = sys.argv[2]

    # get the data we're interested in from the json file.
    # cell_type is the cell_type whose sharing pdf we will build
    # pdf_data is a dictionary that maps a number n (indicating a cell being shared n times)
    # to the proportion of cells that have been shared exactly n times.
    data = json.load(open(json_file))
    pdf_data = data[cell_type]

    # given a pdf, we need a cumulative value to build the cdf
    cumulative_val = 0.0
    y_axis = []
    x_axis = list(range(1, max(int(x) for x in pdf_data.keys()) + 1))
    for i in x_axis:
        # if there is an entry for key i, add it's corresponding value.
        # otherwise add 0.
        pdf_val = pdf_data[str(i)] if str(i) in pdf_data else 0.0
        cumulative_val += pdf_val
        y_axis.append(cumulative_val)

    # scale so that reuslts are between 0 and 1
    y_axis = [v/cumulative_val for v in y_axis]

    plt.bar(x_axis, y_axis, width=1, align="edge")
    plt.show()
