TVM to FuTIL via Relay
======================

This is an in-progress compiler from [TVM][]'s intermediate representation, [Relay][], targeting FuTIL.


Installation
------------

1. Clone the TVM repository with commit hash `ccacb1ec1`):

        git clone --recursive git@github.com:apache/incubator-tvm.git
        cd incubator-tvm && git reset --hard ccacb1ec1

2. Set up to build (the default configuration is fine because we don't need any fancy backends like LLVM or CUDA):

        mkdir build && cd build
        cp ../cmake/config.cmake .

4. Build TVM:

        cmake -G Ninja .. && ninja

5. Install the `tvm` Python package by building a [wheel][]:

        cd ../python && python3 setup.py bdist_wheel
        pip3 install --user dist/tvm-*.whl

6. Install the accompanying `topi` Python package:

        cd ../topi/python && python3 setup.py bdist_wheel
        pip3 install --user dist/topi-*.whl

7. To run the [MLP net][] and [VGG net][] examples, install `pytest`:
        
        pip3 install pytest

8. Install [Dahlia][], which is used when lowering from Relay to FuTIL.
The `fuse` executable is expected to be on your path. Alternatively, it will check to see if the environment variable `$DAHLIA_EXEC` is set. 

Run an Example
--------------

Try this to run a simple example:
```bash
cd futil/frontends/relay
python3 example.py add
```     
Pass the `-h` flag to this script for help.
Pass the `-r` flag to this script to see the Relay IR. Otherwise, we just print the FuTIL output. 


Run the Tests
-------------

The Relay-to-FuTIL compiler has [Runt][] tests in the `tests` directory.
To use them, install Runt:

    cargo install runt

The Relay text format parser requires ANTLR, so also do this:

    pip3 install --user antlr4-python3-runtime==4.7.2

Then, just type `runt` to run the tests.

[vgg net]: https://github.com/apache/incubator-tvm/blob/main/python/tvm/relay/testing/vgg.py 
[mlp net]: https://github.com/apache/incubator-tvm/blob/main/python/tvm/relay/testing/mlp.py
[dahlia]: https://github.com/cucapra/dahlia#set-it-up
[tvm]: https://tvm.apache.org
[tvm-install]: https://tvm.apache.org/docs/install/from_source.html#developers-get-source-from-github
[relay]: https://tvm.apache.org/docs/api/python/relay/index.html
[wheel]: https://packaging.python.org/guides/distributing-packages-using-setuptools/#wheels
[runt]: https://github.com/rachitnigam/runt
