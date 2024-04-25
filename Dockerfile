# Use the official rust image as a parent image.
FROM rust:1.76

# Connect to the Calux repository.
LABEL org.opencontainers.image.source https://github.com/calyxir/calyx

# Install apt dependencies
RUN echo "deb https://repo.scala-sbt.org/scalasbt/debian all main" | tee /etc/apt/sources.list.d/sbt.list && \
    echo "deb https://repo.scala-sbt.org/scalasbt/debian /" | tee /etc/apt/sources.list.d/sbt_old.list && \
    curl -sL "https://keyserver.ubuntu.com/pks/lookup?op=get&search=0x2EE0EA64E40A89B84B2DF73499E82A75642AC823" | apt-key add && \
    apt-get update -y && \
    apt-get install -y jq python3.10 python3-pip python3-venv sbt make autoconf g++ flex bison libfl2 libfl-dev default-jdk ninja-build build-essential cmake autoconf gperf

# Setup python venv to install python dependencies. Python no longer supports installing packages globally into your system
RUN python3 -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

# Install python dependencies
RUN python3 -m pip install numpy flit prettytable wheel hypothesis pytest simplejson cocotb==1.6.2
# Current cocotb-bus has a bug that is fixed in more up to date repo
RUN python3 -m pip install git+https://github.com/cocotb/cocotb-bus.git cocotbext-axi

# Install clang
RUN apt-get install -y clang

# Install Verilator
WORKDIR /home
## TODO(rachit): Don't hardcode the version here
RUN git clone --depth 1 --branch v5.002 https://github.com/verilator/verilator
WORKDIR /home/verilator
RUN autoconf && ./configure && make && make install

# Install Icarus verilog
WORKDIR /home
RUN git clone --depth 1 --branch v12_0 https://github.com/steveicarus/iverilog
WORKDIR /home/iverilog
RUN sh autoconf.sh && ./configure && make && make install

# Install TVM
WORKDIR /home
## TODO(rachit): Do not hardcode here
## NOTE(rachit): Not ideal. We have to clone the entire history of the main branch instead of just a tag.
RUN git clone --single-branch https://github.com/apache/tvm.git tvm
WORKDIR /home/tvm
RUN git checkout v0.10.dev0 && \
    git submodule init && git submodule update
RUN mkdir build
WORKDIR /home/tvm/build
RUN cp ../cmake/config.cmake . && \
    cmake -G Ninja .. && ninja && \
    python3 -m pip install -Iv antlr4-python3-runtime==4.7.2
WORKDIR /home/tvm/python
RUN python3 setup.py bdist_wheel && python3 -m pip install dist/tvm-*.whl

# Install Dahlia
WORKDIR /home
RUN git  clone https://github.com/cucapra/dahlia.git
WORKDIR /home/dahlia
## Checkout specific version. Fetch before checkout because clone might be cached.
RUN git fetch --all && git checkout 9ec9a58
RUN sbt "; getHeaders; assembly"

# Add the Calyx source code from the build context
WORKDIR /home
ADD . calyx
# Build the compiler
WORKDIR /home/calyx
RUN cargo build --all && \
    cargo install vcdump && \
    cargo install runt --version 0.4.1

# Install fud
WORKDIR /home/calyx/fud
RUN FLIT_ROOT_INSTALL=1 flit install --symlink --deps production
RUN mkdir -p /root/.config
ENV PATH=$PATH:/root/.local/bin
ENV PYTHONPATH=/root/.local/lib/python3.9/site-packages:$PYTHONPATH

# Link fud2
WORKDIR /home/calyx
run mkdir -p ~/.local/bin
RUN ln -s /home/calyx/target/debug/fud2 ~/.local/bin/
RUN printf "[calyx]\nbase = \"/home/calyx\"" >> ~/.config/fud2.toml

# Install calyx-py
WORKDIR /home/calyx/calyx-py
RUN FLIT_ROOT_INSTALL=1 flit install --symlink

# Setup fud
RUN fud config --create global.root /home/calyx && \
    fud config stages.dahlia.exec '/home/dahlia/fuse' && \
    fud config stages.calyx.exec '/home/calyx/target/debug/calyx' && \
    fud config stages.interpreter.exec '/home/calyx/target/debug/cider' && \
    fud register ntt -p '/home/calyx/frontends/ntt-pipeline/fud/ntt.py' && \
    fud register mrxl -p '/home/calyx/frontends/mrxl/fud/mrxl.py' && \
    fud register icarus-verilog -p '/home/calyx/fud/icarus/icarus.py'

# Install MrXL
WORKDIR /home/calyx/frontends/mrxl
RUN FLIT_ROOT_INSTALL=1 flit install --symlink

WORKDIR /home/calyx

# Used to make runt cocotb tests happy
ENV LANG=C.UTF-8
