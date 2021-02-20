# Use the official image as a parent image.
FROM rust:latest

# Install dependencies.
RUN cargo install runt vcdump
RUN apt-get update -y && \
    apt-get install -y python3-pip python3-numpy verilator jq
RUN pip3 install flit

# Add source code to a working directory.
WORKDIR /home/calyx
COPY . .

# Build the compiler.
RUN cargo build

# Install and set up fud.
WORKDIR /home/calyx/fud
RUN FLIT_ROOT_INSTALL=1 flit install --symlink
RUN mkdir -p /root/.config
ENV PATH=$PATH:/root/.local/bin
RUN fud config global.futil_directory /home/calyx
WORKDIR /home/calyx
