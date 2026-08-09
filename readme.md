# Oxidised Vector Graphics (OXVG)

[![release](https://img.shields.io/github/v/release/noahbald/oxvg)](https://github.com/noahbald/oxvg) [![npm](https://img.shields.io/npm/v/@oxvg/wasm)](https://www.npmjs.com/~oxvg) [![crate](https://img.shields.io/crates/v/oxvg)](https://crates.io/users/noahbald) [![discord](https://img.shields.io/discord/1385773366396325899)](https://discord.gg/9RudZ7kTGH) [![wiki](https://img.shields.io/badge/docs-home-green)](https://github.com/noahbald/oxvg/wiki)

OXVG is the fastest[^1] SVG toolchain for optimisation, linting, transformation, and manipulation. Usable via CLI and libraries for [Node](https://www.npmjs.com/package/@oxvg/napi), [WASM](https://www.npmjs.com/package/@oxvg/wasm), or Rust.

## Installation

```sh
cargo install oxvg

# View commands
oxvg --help
# Optimise an SVG
oxvg optimise < input.svg > output.svg
```

## 🎯 Tools

The following tools are available in a CLI binary.

### 🪶 Optimiser

> [!TIP]
> You can try out the OXVG optimiser right in your browser using [OXVGUI](https://oxvgui.jonasgeiler.com/), a simple web-based playground built by [Jonas Geiler (@jonasgeiler)](https://github.com/jonasgeiler).


An SVG [optimiser](https://github.com/noahbald/oxvg/wiki/Optimiser) similar to [SVGO](https://github.com/svg/svgo) is available. It can run [up to 50x faster](https://github.com/noahbald/oxvg/wiki/Benchmarks), especially on larger file-sets.

The optimiser is based on and aims for compatibility with SVGO, but it isn't a 1-for-1 replacement. Some plugins may behave differently. See the [SVGO parity](https://github.com/noahbald/oxvg/wiki/Optimiser#svgo-parity) guide to understand the differences and how to migrate your existing configuration.

```sh
# or `oxvg optimize`
cat my-file.svg | oxvg optimise > my-file.optimised.svg
```

https://github.com/user-attachments/assets/b2f54ab5-33de-44e4-aca5-3a269aae4dd6

### 🤖 Actions (Under Development[^2])

<!-- TODO: uncomment when redeployed
> [!TIP]
> You can try out OXVG actions right in your browser using [Vivec](https://oxvg.noahwbaldwin.me/), an integration of actions into a Vi-like web-editor.
>
> It's very early alpha and is limited and rough around the edges.
-->

[Actions](https://github.com/noahbald/oxvg/wiki/Actions) are a set of commands that can be invoked by a program to manipulate an SVG document or pull information from it.
It is comparable to Inkscape's actions, but without any dependency on the UI or rendering.

```sh
cat overlapping-paths.svg | oxvg action -- -select path -path-intersect > merged-paths.svg
```

### 🧹 Linter

A basic [linter](https://github.com/noahbald/oxvg/wiki/Linter) similar to svglint or vnu is available to make catching issues in SVG documents much easier. The linter can report SVG issues directly from the command line or through its language-server integration.

```sh
# Start language-server
oxvg lint serve
# Report diagnostics to stderr
oxvg lint check w3c/ -r
```

<img width="1147" height="334" alt="linting output" src="https://github.com/user-attachments/assets/a5c190e6-b685-4c6e-ba35-1c8bd3578b02" />

### ⚛️ JSX 

A [JSX transformer](https://github.com/noahbald/oxvg/wiki/JSX) to take SVG documents and transform them into JSX components, for use in React, Preact, Native, or any other JSX-compatible language.

This aims to implement SVGR as closely as possible.

```sh
cat my-file.svg | oxvg jsx --template template.jsx | prettier > my-file.svg.jsx
```

## 📖 Libraries

If you're a Rust developer wanting to work with SVGs in your project, we have a set of crates at your disposal.

OXVG's functionality is split into separate crates that can be used independently, depending whether you need to integrate DOM parsing/traversal, path handling, optimisation, manipulation, etc., into your project.

### [Actions](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_actions) (Unstable[^3])

Actions are programmatic commands for manipulating and inspecting SVG documents. 

### [AST](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_ast)

This crate provides a set of types that can be used to implement a DOM similar to that of the browser web standards. Though it's not a 1-to-1 match; it's designed for easily traversing and manipulating the DOM.

There's currently an implementation that can be used with either the xml5ever or the roxmltree parser which can do the following.

- Parse and serialise XML, SVG, and HTML documents
- Commonly used browser API implementations for DOM nodes, elements, attributes, etc.
- An implementation of [selectors](https://docs.rs/selectors/0.26.0/selectors/) for using DOM CSS queries

### [Collections](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_collections)

This crate provides types for SVG content.

- Parsing attributes into structured data
- Enumerators for known elements, attributes, and namespaces

### [Optimiser](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_optimiser)

This is where the jobs (i.e. SVGO plugins) for our optimiser live and can also be used as a library for use in your applications.

### [Path](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_path) (Unstable[^3])

This is a library for parsing, optimising, and serialising path definitions (e.g. `<path d="..." />`).

Please expect some instability as we may add new features to enable simple manipulations for paths in the future.

### [JSX](https://github.com/noahbald/oxvg/tree/main/crates/oxvg_jsx)

This is a library for parsing SVG, optimising, and transforming it into JSX modules.

## Building

You can build the project for yourself by doing the following

```sh
git clone git@github.com:noahbald/oxvg.git
cd oxvg
cargo build --profile release --package oxvg --bin oxvg
# UNIX
./target/release/oxvg --help
# Windows
.\target\release\oxvg.exe --help
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

Thank you to these high quality, open source projects on SVG tooling

- SVGO
- SVGR
- Inkscape
- Linebender

Thank you to these projects for helping make OXVG more popular

- Parcel, for choosing us as a [default optimiser](https://parceljs.org/languages/svg/#minification)

## Licensing

OXVG is open-source and licensed under the [MIT License](./LICENSE)

This project ports or copies code from other open-source (MIT) projects, listed below

- SVGO
- SVGR
- OXC
- Parcel
- Kurbo

[^1]: Fastest I'm aware of; see [benchmarks](https://github.com/noahbald/oxvg/wiki/Benchmarks)
[^2]: CLI commands under development are safe to use but are either incomplete or likely to experience breaking changes in future.
[^3]: Unstable libraries are either incomplete or likely to experience breaking changes in future.
