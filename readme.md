# Oxidised Vector Graphics

[![release](https://img.shields.io/github/v/release/noahbald/oxvg)](https://github.com/noahbald/oxvg) [![npm](https://img.shields.io/npm/v/@oxvg/wasm)](https://www.npmjs.com/~oxvg) [![crate](https://img.shields.io/crates/v/oxvg)](https://crates.io/users/noahbald) [![discord](https://img.shields.io/discord/1385773366396325899)](https://discord.gg/9RudZ7kTGH)

OXVG is an effort to create high-performance SVG tooling.

Bindings for [node](https://www.npmjs.com/package/@oxvg/napi) and [wasm](https://www.npmjs.com/package/@oxvg/wasm) are available through NPM.

You can try out OXVG right in your browser using [OXVGUI](https://oxvgui.jonasgeiler.com/), a simple web-based playground built by [Jonas Geiler (@jonasgeiler)](https://github.com/jonasgeiler).

## 🎯 Tools

The following tools will be available in a CLI binary.

### 🪶 Optimiser

An SVG optimiser similar to [SVGO](https://github.com/svg/svgo) is available and runs [multiple times faster](https://github.com/noahbald/oxvg/wiki/Benchmarks) on some tasks.

The optimiser is based on SVGO, but please be aware that this isn't an exact clone of SVGO and certain differences may be found. If you rely on stability, for the time being we recommend sticking to SVGO.

You can read more about these differences and why in [this wiki page](https://github.com/noahbald/oxvg/wiki/Optimiser#svgo-parity).
Differences for any of the jobs are also documented with each struct/interface's declaration.

### 🤖 Transformer (Planned)

An SVG transformer similar to Inkscape's actions is planned.

### 🧹 Linter (Planned)

A basic linter similar to svglint is planned to make catching issues in SVG documents much easier.

## 📖 Libraries

If you're a Rust developer wanting to work with SVGs in your project, we have a set of crates at your disposal.
As of now though, we're quite unstable and certain crates may be updated, merged, or moved as we see fit.

### [Actions](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_actions) (pre-alpha)

These are where the commands for our transformer will live and will contain a set of actions to manipulate SVGs.

### [AST](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_ast) (beta)

This crate provides a set of traits that can be used to implement a DOM similar to that of the browser web standards. Though it's not a 1-to-1 match; it's designed for easily traversing and manipulating the DOM.

There's currently an implementation that can be used with either the xml5ever or the roxmltree parser which can do the following.

- Parse and serialize XML, SVG, and HTML documents
- Commonly used browser API implementations for DOM nodes, elements, attributes, etc.
- An implementation of [selectors](https://docs.rs/selectors/0.26.0/selectors/) for using DOM CSS queries

The AST is open for extension, so you can implement your own parser or document representation to get it working with the optimiser.

#### [Style](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_ast/src/style.rs)

This crate uses lightningcss to provide some shortcuts for using CSS with our AST.

- Parsing presentation attributes as CSS (hopefully this can be ported to lightningcss)
- Collecting the computed styles of a HTML element

### [Optimiser](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_optimiser) (beta)

This is where the jobs (i.e. SVGO plugins) for our optimiser live and can also be used as a library for use in your applications.

### [Path](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_path) (beta)

This is a library for parsing, optimising, and serialising path definitions (e.g. `<path d="..." />`).

Please expect some instability as we may add new features to enable simple manipulations for paths in the future.

## 💭 Other future ideas

The future potential of this project is still undecided. The following may be available some point in the future.

- NPX bindings
- A web frontend comparable to InkScape
- A TUI frontend

## Building

This project is currently in very early development and doesn't have any distributions yet.
You can run the project for yourself by doing the following

```sh
git clone git@github.com:noahbald/oxvg.git
cargo build --package oxvg
./target/debug/oxvg.exe --help
```

Or you can try running it through `cargo` instead

```sh
cargo run -- --help
```

## Contributing

You're welcome to help out and pick up a [good first issue](https://github.com/noahbald/oxvg/labels/good%20first%20issue) or email me to help.

---

# Inspiration and Thanks

Thank you to the following projects for providing me inspiration to break into the tooling space.

- oxc

Thank you to these high quality, open source projects on SVG tooling

- SVGO
- InkScape

## Licensing

OXVG is open-source and licensed under the [MIT License](./LICENSE)

This project ports or copies code from other open-source projects, listed below

- SVGO
- oxc
