# Queues Library

See the [docs][docs] for more details.

## Installation
To use the queues:
1. Install [flit][flit]
2. Install the `queues` package:
```
    $ cd frontends/queues/
    $ flit install --symlink
``` 

## Converting Tests to Calyx

To convert any of our [randomized tests][testing-harness] to a single Calyx file and their associated data and expect files:

0. Follow the [installation instructions](#installation)
1. Choose a test by picking a `.py` file in [`tests/`][tests-dir]
2. Convert the test to Calyx:
```
python3 <queue_name>_test.py 20000 --keepgoing > <queue_name>_test.futil
```
3. Run the script [`gen_test_data.sh`][gen_test_data.sh] to generate data and expect files:
```
./gen_test_data.sh
```

The files `<queue_name>_test.py`, `<queue_name>_test.data`, and `<queue_name>_test.expect` contain the Calyx program, input data, and expected outputs for the test.

[docs]: https://docs.calyxir.org/frontends/queues.html
[flit]: https://flit.readthedocs.io/en/latest/#install
[testing-harness]: https://docs.calyxir.org/frontends/queues.html#shared-testing-harness
[tests-dir]: ./tests/
[gen_test_data.sh]: ./test_data_gen/gen_test_data.sh