# Calyx Web Demo

This web demo uses `calyx` as a library, compiled to WebAssembly, to provide an
interactive in-browser interface for the Calyx compiler.

## Requirements

You will need [Rust][] and [Node.js][] and [NPM][] or [Yarn][].
To get [WebAssembly support][wasm-qs] in Rust, the easiest way is to use [rustup][] rather than installing Rust with a package manager.

Then install `wasm-pack` with:

```shell
cargo install wasm-pack wasm-bindgen-cli
```

## Build and Run

First, install the dependencies by typing `yarn` or `npm i`.

Then, launch a local web server (which will also rebuild the site when source files change) by typing `yarn start` or `npm start`.

To build standalone web files, use `yarn build` or `npm run build`.

[rust]: https://www.rust-lang.org/install.html
[Node.js]: https://nodejs.org/en/
[npm]: https://nodejs.org/en/
[yarn]: https://yarnpkg.com
[wasm-qs]: https://rustwasm.github.io/docs/wasm-pack/quickstart.html
[rustup]: https://rustup.rs
