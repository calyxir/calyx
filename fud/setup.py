#!/usr/bin/env python3

from setuptools import setup, find_packages
setup(
    name="fud",
    version="0.1",
    description="FuTIL driver tool.",
    url='https://github.com/cucapra/futil/tree/fud/fud',
    packages=find_packages(),
    license='MIT',

    # registers the main function as a command line script
    entry_points={
        "console_scripts": ['fud=src.main:main']
    },

    install_requires=[
        'appdirs',
        'toml'
    ]
)
