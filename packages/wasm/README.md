# Oxidised Vector Graphics for WASM

OXVG is an effort to create high-performance SVG tooling.

It's planned to include transforming, optimising, and linting, all written in Rust.

See the main [readme](https://github.com/noahbald/oxvg/blob/main/readme.md) for more!

## Tools

The following are available through WASM bindings

### 🪶 Optimiser

An SVG optimiser similar to [SVGO](https://github.com/svg/svgo).

#### Examples

Optimise svg with the default configuration

```js
import init, { optimise } from "@oxvg/wasm";

await init(); // must be called in browser context!
const result = optimise(`<svg />`);
```

Or, provide your own config

```js
import { optimise } from "@oxvg/wasm";

// Only optimise path data
const result = optimise(`<svg />`, { convertPathData: {} });
```

Or, extend a preset

```js
import { optimise, extend } from "@oxvg/wasm";

const result = optimise(
    `<svg />`,
    extend("default", { convertPathData: { removeUseless: false } }),
);
```
