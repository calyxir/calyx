
# --- STAGE 1: Dahlia ---
FROM sbtscala/scala-sbt:eclipse-temurin-17.0.4_1.7.1_3.2.0 AS dahlia-install
# Install Dahlia
WORKDIR /home
RUN git clone https://github.com/cucapra/dahlia.git
WORKDIR /home/dahlia
## Checkout specific version. Fetch before checkout because clone might be cached.
RUN git fetch --all && git checkout 9ec9a58
RUN sbt "; getHeaders; assembly"

# --- STAGE 2: Icarus ---
FROM debian:trixie AS icarus-install

RUN apt-get update -y && \
    apt-get install -y jq python3-dev make autoconf g++ flex bison libfl2 libfl-dev default-jdk ninja-build build-essential cmake autoconf gperf clang git
# Install Icarus Verilog
# NOTE(griffin): The final install happens later
WORKDIR /home
RUN git clone --depth 1 --branch v12_0 https://github.com/steveicarus/iverilog
WORKDIR /home/iverilog
RUN sh autoconf.sh && ./configure && make

# --- STAGE 3: TVM ---
FROM debian:trixie AS tvm-install
RUN apt-get update -y && \
    apt-get install -y jq python3-dev make autoconf g++ flex bison libfl2 libfl-dev default-jdk ninja-build build-essential cmake autoconf gperf clang git
# Install TVM
## NOTE(griffin): I can't find a way to shove this install into it's own image
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

# --- STAGE 4: Verilator ---
# NOTE(griffin): The final install happens later
FROM debian:trixie AS verilator-install
RUN apt-get update -y && \
    apt-get install -y jq python3-dev make autoconf g++ flex bison libfl2 libfl-dev default-jdk ninja-build build-essential cmake autoconf gperf clang git
# Install Verilator
WORKDIR /home
## TODO(rachit): Don't hardcode the version here
RUN git clone --depth 1 --branch v5.002 https://github.com/verilator/verilator
WORKDIR /home/verilator
RUN autoconf && ./configure && make


# --- STAGE 5: Calyx ---
# Use the official rust image as a parent image.
FROM rust:1.90 AS calyx
# Used to make runt cocotb tests happy
ENV LANG=C.UTF-8

# Connect to the Calyx repository.
LABEL org.opencontainers.image.source=https://github.com/calyxir/calyx

# Install apt dependencies
RUN apt-get update -y && \
    apt-get install -y jq python3-dev make autoconf g++ flex bison libfl2 libfl-dev default-jdk ninja-build build-essential cmake autoconf gperf clang


# Install Firtool
WORKDIR /home
RUN curl -L https://github.com/llvm/circt/releases/download/firtool-1.75.0/firrtl-bin-linux-x64.tar.gz | tar -xz \
    && chmod +x /home/firtool-1.75.0/bin/firtool

COPY --from=verilator-install /home/verilator/ /home/verilator
COPY --from=dahlia-install /home/dahlia/fuse /home/dahlia/fuse
COPY --from=icarus-install /home/iverilog /home/iverilog
COPY --from=tvm-install /home/tvm /home/tvm

# Do the final install for these two in the main image even though the rest is
# built in a different stage
WORKDIR /home/iverilog
RUN make install

WORKDIR /home/verilator
RUN make install

WORKDIR /home
RUN cargo install vcdump && \
    cargo install runt --version 0.4.1

COPY --from=ghcr.io/astral-sh/uv:0.10.2 /uv /uvx /bin/
ENV UV_COMPILE_BYTECODE=1
ENV UV_LINK_MODE=copy

# Add the Calyx source code from the build context
WORKDIR /home
ADD . calyx
# Build the compiler
WORKDIR /home/calyx
RUN cargo build --workspace

# Link fud2
WORKDIR /home/calyx
RUN ln -s /home/calyx/target/debug/fud2 /bin/

# Prepare the fud2 config
RUN mkdir ~/.config
RUN printf "dahlia = \"/home/dahlia/fuse\"\n" >> ~/.config/fud2.toml
RUN printf "[calyx]\nbase = \"/home/calyx\"\n" >> ~/.config/fud2.toml
RUN printf "[firrtl]\nfirtool = \"/home/firtool-1.75.0/bin/firtool\"\n" >> ~/.config/fud2.toml

RUN fud2 env init
# NOTE(griffin): Hardcoding this is not ideal but I currently don't have any
# better ideas. This env var is important to make sure everything lands in the
# right spot
ENV VIRTUAL_ENV=/root/.local/share/fud2/venv
ENV PATH="${VIRTUAL_ENV}/bin:$PATH"
WORKDIR /home/tvm/python
RUN uv pip install antlr4-python3-runtime==4.7.2 .

# Setup fud
RUN fud config --create global.root /home/calyx && \
    fud config stages.dahlia.exec '/home/dahlia/fuse' && \
    fud config stages.calyx.exec '/home/calyx/target/debug/calyx' && \
    fud config stages.interpreter.exec '/home/calyx/target/debug/cider' && \
    fud register ntt -p '/home/calyx/frontends/ntt-pipeline/fud/ntt.py' && \
    fud register mrxl -p '/home/calyx/frontends/mrxl/fud/mrxl.py' && \
    fud register icarus-verilog -p '/home/calyx/fud/icarus/icarus.py'

RUN uv pip install numpy==1.26.4 cocotb==1.6.2 pytest \
    git+https://github.com/cocotb/cocotb-bus.git cocotbext-axi

WORKDIR /home/calyx
