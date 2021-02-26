# TVM Relay

[TVM][] is a compiler for machine learning frameworks that can 
optimize and target kernels to several different backends. [Relay][]
is a high level intermediate representation for the TVM framework. 
The goal of Relay is to replace old computation graph based 
IRs with a more expressive IR that can be optimized for many targets. 
More information can be found in [this paper][roesch-etal].

The TVM Relay frontend lives in the [relay-lang][] folder in the
Calyx repository and generates Calyx components from the Relay
intermediate representation.

## Installation

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

7. Install ANTLR v4.7.2 (required for the Relay text format parser):

        pip3 install -Iv antlr4-python3-runtime==4.7.2

8. To run the [MLP net][] and [VGG net][] examples, install `pytest`:
        
        pip3 install pytest
        
9. Install [Dahlia][], which is used when lowering Relay call nodes to Calyx.

Run an Example
--------------

Try this to run a simple example:

    bash
    cd calyx/frontends/relay
    python3 example.py tensor_add
     
- `-h`: Help option; shows available examples.
- `-r`: Dumps the Relay IR. Otherwise, it dumps the Calyx output. 

[relay-lang]: https://github.com/cucapra/calyx/tree/master/frontends/relay
[roesch-etal]: https://arxiv.org/abs/1904.08368
[vgg net]: https://github.com/apache/incubator-tvm/blob/main/python/tvm/relay/testing/vgg.py 
[mlp net]: https://github.com/apache/incubator-tvm/blob/main/python/tvm/relay/testing/mlp.py
[dahlia]: https://github.com/cucapra/dahlia#set-it-up
[tvm]: https://tvm.apache.org
[tvm-install]: https://tvm.apache.org/docs/install/from_source.html#developers-get-source-from-github
[relay]: https://tvm.apache.org/docs/api/python/relay/index.html
[wheel]: https://packaging.python.org/guides/distributing-packages-using-setuptools/#wheels
