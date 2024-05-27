# Oxidised Vector Graphics

This project is an effort to improve the SVG tooling with browser-grade parsing, transforming, optimising, and linting.

## Features

The following is a high-level overview of planned features

- [ ] SVG Transformer/Optimiser

  - [ ] Implement all built-in SVGO plugins
  - [ ] Implement all InkScape actions
  - [ ] Implement new optimisations
    - [ ] Non-destructively delete useless nodes
    - [ ] Crop partially visible paths

- [ ] SVG linter

  - [ ] Implement all built-in svglint rules

- [ ] NPX & NPM bindings

And maybe in the future???

- Web frontend comparable to InkScape
- TUI frontend
- Light-weight alternate format to SVG

## Building

This project is currently in very early development and doesn't have any distributions yet.
You can run the project for yourself by doing the following

```sh
git clone git@github.com:noahbald/oxvg.git
cargo build --package oxvg
./target/debug/oxvg.exe --help
```

## Goals

For me, this is a learning excercise, for others this may end up being a tool. These goals may be challenged as the project grows, but to me our goal is to

- Write code that is easily understood by beginners to Rust
- Focus on optimisation and quality

### Architecture

This project will probably be shifted around a lot as the architecture is fleshed out. The following should ideally come to into place.

- Break components of the tooling into workspaces
- All public functions should have testing

# Inspiration and Thanks

Thank you to the following projects for providing me inspiration to break into the tooling space.

- oxc

Thank you to these high quality, open source projects on SVG tooling

- SVGO
- InkScape

## Licensing

This project partially copies patterns from the following libraries

- SVGO
- oxc
