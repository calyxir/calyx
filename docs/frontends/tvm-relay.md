# TVM Relay

[TVM][] is a compiler for machine learning frameworks that can
optimize and target kernels to several different backends. [Relay][]
is a high level intermediate representation for the TVM framework.
The goal of Relay is to replace old computation graph based
IRs with a more expressive IR.
More information can be found in [this paper][roesch-etal].

The TVM Relay frontend lives in the [relay-lang][] folder in the
Calyx repository and generates Calyx components from the Relay
intermediate representation.

## Installation

1. Clone the TVM repository and checkout the tag `v0.10.dev0`:

        git clone git@github.com:apache/incubator-tvm.git
        cd incubator-tvm && git checkout v0.10.dev0
        git submodule init && git submodule update

2. Set up to build (the default configuration is fine because we don't need any fancy backends like LLVM or CUDA):

        mkdir build && cd build
        cp ../cmake/config.cmake .

3. Build TVM:

        cmake -G Ninja .. && ninja

4. Install the `tvm` Python package by building a [wheel][]:

        cd ../python && python3 setup.py bdist_wheel
        pip3 install --user dist/tvm-*.whl

   > If you get an error with `shutil`, try deleting the `python/` directory, restoring it, and rerunning the above command: `cd .. && rm -rf python && git checkout -- python`
   > If you are on MacOS - Big Sur and are getting an error similar to "(wheel).whl is not a supported wheel on this platform", try changing part of the wheel's filename from 11_0 to 10_9. See this github [issue](https://github.com/apple/tensorflow_macos/issues/46) for more information.

6. Install ANTLR v4.7.2 (required for the Relay text format parser):

        pip3 install -Iv antlr4-python3-runtime==4.7.2

7. To run the [MLP net][] and [VGG net][] examples, install `pytest`:

        pip3 install pytest

8. Install [Dahlia][], which is used when lowering Relay call nodes to Calyx.

10. Install the [calyx-py](../calyx-py.md) library.

Run an Example
--------------

Try this to run a simple example:

    cd calyx/frontends/relay
    python3 example.py tensor_add

- `-h`: Help option; shows available examples.
- `-r`: Dumps the Relay IR. Otherwise, it dumps the Calyx output.


Simulate an ONNX Model
--------------

A simple script is provided to run an Open Neural Network Exchange (ONNX) model.
In addition to installing TVM Relay above, you'll need the following PIP installations
for ONNX simulation and image pre-processing:

    pip3 install opencv-python Pillow mxnet onnx simplejson

For example, we can simulate the LeNet ONNX model found [here][lenet] using the following command:

    python3 frontends/relay/onnx_to_calyx.py \
    -n "lenet" \
    -d "MNIST" \
    -i "/path/to/image.png" \
    -onnx "/path/to/model.onnx" \
    -o calyx

- `-n`: The name of the input net. This is mostly used for naming the output files.
- `-d`: The dataset for which the input will be classified against. This is necessary to
determine what preprocessing should be done on the image. e.g. `"mnist"` or `"imagenet"`.
- `-i`: The file path to the input image which you want classified.
- `-onnx`: The file path to the ONNX model.
- `-o`: The type of output.
    1. `tvm`: Executes the ONNX model using the TVM executor. Prints the final softmax value
    to console. No postprocessing is conducted.
    2. `relay`: Output a file with the corresponding Relay program. `<net_name>.relay`
    3. `calyx`: Output a `.data` file and Calyx program for simulation. `<net_name>.futil`, `<net_name>.data`
    4. `all`: All the above.
- `-s`: This is an optional boolean argument that signifies `save_mem`, and is set to true by default. If this flag is set to true, then it will produce a Calyx
design that requires less internal memory usage compared to the design that is produced when this flag is false.


[lenet]: https://github.com/ekut-es/pico-cnn/blob/master/data/lenet/lenet.onnx
[relay-lang]: https://github.com/calyxir/calyx/tree/master/frontends/relay
[roesch-etal]: https://arxiv.org/abs/1904.08368
[vgg net]: https://github.com/apache/incubator-tvm/blob/main/python/tvm/relay/testing/vgg.py
[mlp net]: https://github.com/apache/incubator-tvm/blob/main/python/tvm/relay/testing/mlp.py
[dahlia]: https://github.com/cucapra/dahlia#set-it-up
[onnx]: https://onnx.ai/
[tvm]: https://tvm.apache.org
[tvm-install]: https://tvm.apache.org/docs/install/from_source.html#developers-get-source-from-github
[relay]: https://tvm.apache.org/docs/api/python/relay/index.html
[wheel]: https://packaging.python.org/guides/distributing-packages-using-setuptools/#wheels
