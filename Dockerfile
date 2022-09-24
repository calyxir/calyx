# Use the official image as a parent image.
FROM rust:latest

# Connect to the Calux repository.
LABEL org.opencontainers.image.source https://github.com/cucapra/calyx

# Install apt dependencies
## Add SBT source
RUN echo "deb https://repo.scala-sbt.org/scalasbt/debian all main" | tee /etc/apt/sources.list.d/sbt.list && \
    echo "deb https://repo.scala-sbt.org/scalasbt/debian /" | tee /etc/apt/sources.list.d/sbt_old.list && \
    curl -sL "https://keyserver.ubuntu.com/pks/lookup?op=get&search=0x2EE0EA64E40A89B84B2DF73499E82A75642AC823" | apt-key add
RUN apt-get update -y
RUN apt-get install -y jq python3.9 python3-pip sbt

# Install python dependencies
RUN python3 -m pip install numpy flit prettytable wheel hypothesis pytest simplejson

# Clone the Calyx repository
WORKDIR /home
RUN git clone https://github.com/cucapra/calyx.git calyx

# Install Verilator
WORKDIR /home
RUN apt-get install -y git make autoconf g++ flex bison libfl2 libfl-dev
RUN cat calyx/versions/verilator
## TODO(rachit): Don't hardcode the version here
RUN git clone --depth 1 --branch v4.224 https://github.com/verilator/verilator
WORKDIR /home/verilator
RUN autoconf && ./configure && make && make install

# Install Icarus verilog

# Install Dahlia
WORKDIR /home
RUN git clone https://github.com/cucapra/dahlia.git
WORKDIR /home/dahlia
RUN apt-get install -y default-jdk
RUN sbt "; getHeaders; assembly"

# Install TVM
WORKDIR /home
RUN apt-get install -y ninja-build build-essential cmake
## TODO(rachit): Do not hardcode here
## NOTE(rachit): Not ideal. We have to clone the entire history of the main branch instead of just a tag.
RUN git clone --single-branch https://github.com/apache/tvm.git tvm
WORKDIR /home/tvm
RUN git checkout ccacb1ec1
RUN git submodule init && git submodule update
RUN mkdir build
WORKDIR /home/tvm/build
RUN cp ../cmake/config.cmake .
RUN cmake -G Ninja .. && ninja
RUN python3 -m pip install -Iv antlr4-python3-runtime==4.7.2
WORKDIR /home/tvm/python
RUN python3 setup.py bdist_wheel && python3 -m pip install --user dist/tvm-*.whl
WORKDIR /home/tvm/topi/python
RUN python3 setup.py bdist_wheel && python3 -m pip install --user dist/topi-*.whl

# Install rust tools
WORKDIR /home
RUN cargo install runt --version $(grep ^ver calyx/runt.toml | awk '{print $3}' | tr -d '"')
RUN cargo install vcdump

# Build the compiler.
WORKDIR /home/calyx
RUN cargo build --all

# Install fud
WORKDIR /home/calyx/fud
RUN FLIT_ROOT_INSTALL=1 flit install --symlink
RUN mkdir -p /root/.config
ENV PATH=$PATH:/root/.local/bin

# Setup fud
RUN fud config global.futil_directory /home/calyx && \
    fud config stages.dahlia.exec '/home/dahlia/fuse' && \
    fud config stages.futil.exec '/home/calyx/target/debug/futil' && \
    fud config stages.interpreter.exec '/home/calyx/target/debug/interp' && \
    fud register ntt -p '/home/calyx/frontends/ntt-pipeline/fud/ntt.py'
WORKDIR /home/calyx
