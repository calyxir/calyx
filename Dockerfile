# Use the official rust image as a parent image.
FROM rust:1.76

# Connect to the Calyx repository.
LABEL org.opencontainers.image.source https://github.com/calyxir/calyx

# Install apt dependencies
RUN echo "deb https://repo.scala-sbt.org/scalasbt/debian all main" | tee /etc/apt/sources.list.d/sbt.list && \
    echo "deb https://repo.scala-sbt.org/scalasbt/debian /" | tee /etc/apt/sources.list.d/sbt_old.list && \
    curl -sL "https://keyserver.ubuntu.com/pks/lookup?op=get&search=0x2EE0EA64E40A89B84B2DF73499E82A75642AC823" | apt-key add && \
    apt-get update -y && \
    apt-get install -y jq python3-dev sbt make autoconf g++ flex bison libfl2 libfl-dev default-jdk ninja-build build-essential cmake autoconf gperf clang

# Install uv and create a virtualenv
WORKDIR /home
COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/
ENV UV_COMPILE_BYTECODE=1
ENV UV_LINK_MODE=copy
RUN uv venv
ENV PATH="/home/.venv/bin:$PATH"

# Install python dependencies:
# * cocotb==1.6.2 seems to be for Xilinx cocotb tests
# * Need to pin the numpy version since there are TVM issues with versions 2 and above
# * Current cocotb-bus has a bug that is fixed in more up to date repo
# * Vcdvcd for profiling
# Someday, all of these should come from a pyproject.toml.
RUN uv pip install numpy==1.26.4 prettytable wheel hypothesis pytest \
    simplejson cocotb==1.6.2 \
    git+https://github.com/cocotb/cocotb-bus.git cocotbext-axi \
    vcdvcd

# Install Verilator
WORKDIR /home
## TODO(rachit): Don't hardcode the version here
RUN git clone --depth 1 --branch v5.002 https://github.com/verilator/verilator
WORKDIR /home/verilator
RUN autoconf && ./configure && make && make install

# Install Icarus Verilog
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
    cmake -G Ninja .. && ninja
WORKDIR /home/tvm/python
RUN uv pip install antlr4-python3-runtime==4.7.2 .

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
RUN cargo build --workspace && \
    cargo install vcdump && \
    cargo install runt --version 0.4.1

# Install fud
WORKDIR /home/calyx
RUN uv pip install ./fud
RUN mkdir -p /root/.config

# Link fud2
WORKDIR /home/calyx
RUN mkdir -p ~/.local/bin
RUN ln -s /home/calyx/target/debug/fud2 ~/.local/bin/
RUN printf "dahlia = \"/home/dahlia/fuse\"\n" >> ~/.config/fud2.toml
RUN printf "[calyx]\nbase = \"/home/calyx\"\n" >> ~/.config/fud2.toml

# Setup fud
RUN fud config --create global.root /home/calyx && \
    fud config stages.dahlia.exec '/home/dahlia/fuse' && \
    fud config stages.calyx.exec '/home/calyx/target/debug/calyx' && \
    fud config stages.interpreter.exec '/home/calyx/target/debug/cider' && \
    fud register ntt -p '/home/calyx/frontends/ntt-pipeline/fud/ntt.py' && \
    fud register mrxl -p '/home/calyx/frontends/mrxl/fud/mrxl.py' && \
    fud register icarus-verilog -p '/home/calyx/fud/icarus/icarus.py'

# Install MrXL
WORKDIR /home/calyx
RUN uv pip install ./frontends/mrxl

WORKDIR /home/calyx

# Used to make runt cocotb tests happy
ENV LANG=C.UTF-8
