import json
import sys
import matplotlib.pyplot as plt
'''
Takes in a pdf of sharing frequences and plots the corresponding cdf graph.
Usually, you will generate the json by passing `-x cell-share:print-share-freqs=file-name`
as a flag.
'''


def update_k_v(dic, k, v):
    '''
    Updates dic such that k's corresponding value is increased by v.
    '''
    dic[k] = dic.get(k, 0) + v


if __name__ == "__main__":
    assert (len(sys.argv) == 3), "please provide an input json file name and a cell_type"
    # should contain the data we're interested in
    json_file = sys.argv[1]
    # should contain the cell type that we want to build a cdf for
    cell_type = sys.argv[2]

    # get the data we're interested in from the json file.
    # data is essentailly a bunch of nested dictionaries.
    # It basically maps a number n to the number of cells shared exactly n
    # times, but the data is organized by both which component the sharing is
    # happening in, and cell type
    data = json.load(open(json_file))

    # this input dic will have string keys that we want to turn into ints
    for comp_map in data.values():
        for cell_type in comp_map:
            comp_map[cell_type] = {int(k): v for k, v in comp_map[cell_type].items()}

    # sharing data for the "main" component
    main_data = data["main"]

    # data for sharing frequencies across all components
    # should count the number of times the physical register is actually shared
    total_data = {}

    # go through each component and check the sharing frequencies of each cell_type
    for comp_name in data:
        # if cell_type is shared in component with comp_name, then we will
        # have `new_data`, which we want to add to `total_data``
        if cell_type in data[comp_name]:
            freq_map = data[comp_name][cell_type]
            new_data = {}
            if comp_name in main_data:
                # if comp is shared in the main component, we need to take
                # that into account when we make our `new_data`
                # note that we only take into account components that are shared
                # in the main component. we don't currently take into account
                # subcomponents shared in other subcomponents
                comp_map = main_data[comp_name]
                for (in_component_shared, in_component_num_cells) in freq_map.items():
                    for (component_shared, component_num_cells) in comp_map.items():
                        new_shared = in_component_shared * component_shared
                        new_num_cells = in_component_num_cells * component_num_cells
                        new_data[new_shared] = new_data.get(
                            new_shared, 0) + new_num_cells
            else:
                # otherwise, we can just use the freq_map as is
                new_data = freq_map
            # updating total_data with the `new_data` we just collected
            for (k, v) in new_data.items():
                total_data[k] = total_data.get(k, 0) + v

    # given a sharing frequencies, we need cumulative frequencies to build a cdf
    cumulative_val = 0.0
    y_axis = []
    x_axis = list(range(1, max(int(x) for x in total_data.keys()) + 1))
    for i in x_axis:
        # if there is an entry for key i, add it's corresponding value.
        # otherwise add 0.
        pdf_val = total_data[i] if i in total_data else 0.0
        cumulative_val += pdf_val
        y_axis.append(cumulative_val)

    # scale so that reuslts are between 0 and 1
    y_axis = [v/cumulative_val for v in y_axis]

    plt.bar(x_axis, y_axis, width=1, align="edge")
    plt.show()
