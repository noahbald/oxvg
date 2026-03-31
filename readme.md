# Oxidised Vector Graphics

[![release](https://img.shields.io/github/v/release/noahbald/oxvg)](https://github.com/noahbald/oxvg) [![npm](https://img.shields.io/npm/v/@oxvg/wasm)](https://www.npmjs.com/~oxvg) [![crate](https://img.shields.io/crates/v/oxvg)](https://crates.io/users/noahbald) [![discord](https://img.shields.io/discord/1385773366396325899)](https://discord.gg/9RudZ7kTGH) [![wiki](https://img.shields.io/badge/docs-home-green)](https://github.com/noahbald/oxvg/wiki)

OXVG is an effort to create high-performance SVG tooling.

Bindings for [node](https://www.npmjs.com/package/@oxvg/napi) and [wasm](https://www.npmjs.com/package/@oxvg/wasm) are available through NPM.

## 🎯 Tools

The following tools will be available in a CLI binary.

### 🪶 Optimiser

> [!TIP]
> You can try out the OXVG optimiser right in your browser using [OXVGUI](https://oxvgui.jonasgeiler.com/), a simple web-based playground built by [Jonas Geiler (@jonasgeiler)](https://github.com/jonasgeiler).


An SVG optimiser similar to [SVGO](https://github.com/svg/svgo) is available and runs [multiple times faster](https://github.com/noahbald/oxvg/wiki/Benchmarks) on some tasks.

The optimiser is based on SVGO, but please be aware that this isn't an exact clone of SVGO and certain differences may be found. If you rely on stability, for the time being we recommend sticking to SVGO.

You can read more about these differences and why in [this wiki page](https://github.com/noahbald/oxvg/wiki/Optimiser#svgo-parity).
Differences for any of the jobs are also documented with each struct/interface's declaration.

### 🤖 Actions (Under Development)

> [!TIP]
> You can try out OXVG actions right in your browser using [Vivec](https://oxvg.noahwbaldwin.me/), an integration of actions into a Vi-like web-editor.

[Actions](https://github.com/noahbald/oxvg/wiki/Actions) are a set of commands that can be invoked by a program to manipulate an SVG document or pull information from it.
It is comparable to InkScape's actions, but without any dependency on the UI or rendering.

### 🧹 Linter

A basic [linter](https://github.com/noahbald/oxvg/wiki/Linter) similar to svglint or vnu is available to make catching issues in SVG documents much easier. It's accessible as a printer or a language server.

<img width="1147" height="334" alt="image" src="https://github.com/user-attachments/assets/a5c190e6-b685-4c6e-ba35-1c8bd3578b02" />

## 📖 Libraries

If you're a Rust developer wanting to work with SVGs in your project, we have a set of crates at your disposal.
As of now though, some are unstable and may be updated, merged, or moved as we see fit.

### [Actions](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_actions) (Unstable)

These are where the commands for our transformer will live and will contain a set of actions to manipulate SVGs.

### [AST](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_ast)

This crate provides a set of types that can be used to implement a DOM similar to that of the browser web standards. Though it's not a 1-to-1 match; it's designed for easily traversing and manipulating the DOM.

There's currently an implementation that can be used with either the xml5ever or the roxmltree parser which can do the following.

- Parse and serialize XML, SVG, and HTML documents
- Commonly used browser API implementations for DOM nodes, elements, attributes, etc.
- An implementation of [selectors](https://docs.rs/selectors/0.26.0/selectors/) for using DOM CSS queries

### [Collections](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_collections)

This crate provides types and meta-types for SVG content.

- Parsing attributes into structured data
- Enumerators for known element, attributes, and namespaces

### [Optimiser](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_optimiser)

This is where the jobs (i.e. SVGO plugins) for our optimiser live and can also be used as a library for use in your applications.

### [Path](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_path) (Unstable)

This is a library for parsing, optimising, and serialising path definitions (e.g. `<path d="..." />`).

Please expect some instability as we may add new features to enable simple manipulations for paths in the future.

## Building

This project is currently in very early development and doesn't have any distributions yet.
You can run the project for yourself by doing the following

```sh
git clone git@github.com:noahbald/oxvg.git
cd oxvg
cargo build --profile release --package oxvg
./target/release/oxvg.exe --help
```

Or you can install it through `cargo`

```sh
cargo install oxvg
oxvg --help
```

## Contributing

You're welcome to help out and pick up a [good first issue](https://github.com/noahbald/oxvg/labels/good%20first%20issue) or email me to help.

[Contributing](https://github.com/noahbald/oxvg/wiki/Contributing) and [architecture](https://github.com/noahbald/oxvg/wiki/Architecture) guides are available as well.

---

# Inspiration and Thanks

Thank you to the following projects for providing me inspiration to break into the tooling space.

- oxc

Thank you to these high quality, open source projects on SVG tooling

- SVGO
- InkScape

Thank you to these dependents for helping make OXVG more popular

- Parcel, for choosing us as a [default optimiser](https://parceljs.org/languages/svg/#minification)

## Licensing

OXVG is open-source and licensed under the [MIT License](./LICENSE)

This project ports or copies code from other open-source projects, listed below

- SVGO
- oxc
