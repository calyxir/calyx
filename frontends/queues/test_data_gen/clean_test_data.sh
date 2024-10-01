cd "$(dirname "$0")/../tests"

rm -rf *.data *.expect

cd binheap/
for test in fifo_test pifo_test stable_binheap_test; do
    rm -rf $test.data $test.expect
done
rm -rf round_robin/*.data round_robin/*.expect
rm -rf strict/*.data strict/*.expect

cd ../round_robin
rm -rf *.data *.expect

cd ../strict
rm -rf *.data *.expect
