#!/usr/bin/env python3

from setuptools import setup, find_packages
setup(
    name="fud",
    version="0.1",
    packages=find_packages(),

    # registers the main function as a command line script
    entry_points={
        "console_scripts": ['fud=src.main:main']
    }
)
