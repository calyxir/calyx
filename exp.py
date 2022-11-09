import json
import sys

f = open('a.json')

# returns JSON object as
# a dictionary
data = json.load(f)

print(data)

type = sys.argv[1]


cdf_data = data[type]

num = [int(x) for x in cdf_data.keys()]

prev_value = 0.0
y_axis = []
x_axis = range(1, max(num) + 1)
for i in x_axis:
    if str(i) in cdf_data:
        y_dat = cdf_data[str(i)]
        prev_value = y_dat
    else:
        y_dat = prev_value
    y_axis.append(y_dat)

print(y_axis)
