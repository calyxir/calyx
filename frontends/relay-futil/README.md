TVM to FuTIL via Relay
======================

This is an in-progress compiler from [TVM][]'s intermediate representation, [Relay][], targeting FuTIL.


Installation
------------

You will need to install TVM—and we depend on the latest source (unreleased changes for 0.7). There are [official instructions][tvm-install], but these might work for you:

1. Clone the TVM repository (success was once attained with revision `ccacb1ec1`):

        git clone --recursive git@github.com:apache/incubator-tvm.git
        cd incubator-tvm && git reset --hard ccacb1ec1

2. Set up to build (the default configuration is fine because we don't need any fancy backends like LLVM or CUDA):

        mkdir build && cd build
        cp ../cmake/config.cmake .

4. Build (takes about 9 minutes on my MacBook Pro):

        cmake -G Ninja .. && ninja

5. Install the `tvm` Python package by building a [wheel][]:

        cd ../python
        python3 setup.py bdist_wheel
        pip3 install --user dist/tvm-*.whl

6. Install the accompanying `topi` Python package:

        cd ../topi/python
        python3 setup.py bdist_wheel
        pip3 install --user dist/topi-*.whl


Run an Example
--------------

Try this to run a simple example:
```bash
cd futil/frontends/relay-futil
python3 example.py
```     

Pass the `-r` flag to this script to see the Relay code. Otherwise, we just print the FuTIL code. There is also an `-o` flag to try optimizing the Relay code a little bit.

You can specify the name of an example as a command-line argument. Currently, the only option is `identity`.


Run the Tests
-------------

The Relay-to-FuTIL compiler has [Runt][] tests in the `tests` directory.
To use them, install Runt:

    cargo install runt

The Relay text format parser requires ANTLR, so also do this:

    pip3 install --user antlr4-python3-runtime==4.7.2

Then, just type `runt` to run the tests.


[tvm]: https://tvm.apache.org
[tvm-install]: https://tvm.apache.org/docs/install/from_source.html#developers-get-source-from-github
[relay]: https://tvm.apache.org/docs/api/python/relay/index.html
[wheel]: https://packaging.python.org/guides/distributing-packages-using-setuptools/#wheels
[runt]: https://github.com/rachitnigam/runt
