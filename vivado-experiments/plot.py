#!/usr/bin/env python3

import altair as alt
from pathlib import Path
import json
import pandas as pd
import altair_viewer

alt.renderers.enable('altair_viewer')

source = pd.read_csv('7_31.csv')
chart = alt.Chart(source.sort_values(by='benchmark')).mark_bar().encode(
    x='type:O',
    y='lut:Q',
    column=alt.Column('benchmark:O',
                     title="",
                     header=alt.Header(labelAngle=-20)),
    color='type:O',
    tooltip=['lut']
).interactive()

chart.save("7_31.html")
altair_viewer.show(chart)
