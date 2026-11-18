circt/build/bin/firtool .fud2/tmp-out.fir --ir-hw -o design.mlir
echo step1

circt/build/bin/arcilator design.mlir --state-file=mapping.json -o simulator.ll
echo step2

python3 circt/tools/arcilator/arcilator-header-cpp.py mapping.json > header.h
echo step3

sed 's/)extern/)\n\nextern/g' header.h > header_fixed.h

# Added the warning suppression flag here
clang -Wno-override-module -c simulator.ll -o hardware_model.o
echo step4

clang++ -I circt/tools/arcilator driver.cpp hardware_model.o -o simulate
echo step5

# Using the cider converter to dump the hex data into a directory
target/debug/cider-data-converter --to dat --output-path sim_data_dir driver.futil.data

# The converter names the file after the JSON key ("mem"), so we point the simulator there
./simulate sim_data_dir/mem.dat