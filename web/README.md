# Futil Web Demo ![Deploy](https://github.com/cucapra/futil-site/workflows/Deploy/badge.svg)
This uses `calyx` as a library to provide an interactive
web demo for the Futil compiler. Futil is compiled to webassembly
and wrapped in simple javascript that interfaces with the compiler.

## Building
For now, this repository uses a git checkout of the [Futil repository](https://github.com/cucapra/futil).

### Setup Build Environment

You need [Rust](https://www.rust-lang.org/install.html) and [Node.js and NPM](https://www.npmjs.com/get-npm) installed.
To get [WebAssembly support](https://rustwasm.github.io/wasm-pack/book/quickstart.html), the easiest way is to install Rust with [rustup](https://rustup.rs) as opposed to a package manager.

Then install `wasm-pack` with:

``` shell
cargo install wasm-pack wasm-bindgen-cli
```

Now you are ready to build and run the web demo. First, install the `npm` dependencies with:

``` shell
npm i
```

### Compiling and Testing

Run a test web server with:

``` shell
npm start
```

The server will automatically refresh if you change any of the JavaScript or Rust code.
You can build standalone files with:

``` shell
npm run build
```
