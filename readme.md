# Oxidised Vector Graphics

This project is an effort to improve the SVG tooling with a unified parser, transformer, optimiser, and linter.

## Features

The following is a high-level overview of planned features

- [ ] SVG parser

  - [ ] Full XML 1.1 Compatibility
  - [ ] Full SVG 1.2 Compatibility
  - [ ] Full SVG 2.0 Compatibility
  - [ ] Testing
  - [ ] Human readable & understandable error-reporting
  - [ ] O(n) time complexity

- [ ] SVG Transformer/Optimiser

  - [ ] Implement all built-in SVGO plugins
  - [ ] Implement all InkScape actions
  - [ ] Implement new optimisations
    - [ ] Non-destructively delete useless nodes

- [ ] SVG linter

  - [ ] Implement all built-in svglint rules

- [ ] NPX & NPM bindings

And maybe in the future???

- Web frontend comparable to InkScape
- TUI frontend
- Light-weight alternate format to SVG

## Goals

For me, this is a learning excercise, for others this may end up being a tool. These goals may be challenged as the project grows, but to me our goal is to

- Accurately implement the specifications for XML and SVG
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
