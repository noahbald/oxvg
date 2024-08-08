# Oxidised Vector Graphics

This project is an effort to improve the SVG tooling with browser-grade parsing, transforming, optimising, and linting.

Hopefully this project will be useful to some as a back-end applications competing with Adobe Illustrator or InkScape.

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

### Progress

Please check out the following milestones to see how the project is tracking

- [Requirements for 0.0.1](https://github.com/noahbald/oxvg/milestone/1)

### SVGO Parity

Oxvg aims to be as close as possible to SVGO while providing a more consistent configuration system.

#### Functional Differences

- **Configuration Structure**: To improve the simplicity of oxvg as a Rust program, the configuration structure is somewhat different. A migration tool will eventually be made to make switching over easier.
- **Doesn't support valueless attributes**: Attributes formatted alike `<svg attr />` is valid HTML but not XML. Because of oxvg's dependencies, invalid XML syntax is not supported and will be converted to `<svg attr="" />`.
- **Numerical cleanup**: Unlike SVGO, we include `d` in the type of attributes that can be rounded

## Building

This project is currently in very early development and doesn't have any distributions yet.
You can run the project for yourself by doing the following

```sh
git clone git@github.com:noahbald/oxvg.git
cargo build --package oxvg
./target/debug/oxvg.exe --help
```

## Goals

For me, this is a learning exercise, for others this may end up being a tool. These goals may be challenged as the project grows, but to me our goal is to

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
