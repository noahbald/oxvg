# Oxidised Vector Graphics for WASM

OXVG is the fastest SVG toolchain for optimisation, linting, transformation, and manipulation. Usable via CLI and libraries for [Node](https://www.npmjs.com/package/@oxvg/napi), [WASM](https://www.npmjs.com/package/@oxvg/wasm), or Rust.

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

You can even make use of your existing SVGO config

```js
import { optimise, convertSvgoConfig } from "@oxvg/wasm";
import { config } from "./svgo.config.js";

const result = optimise(`<svg />`, convertSvgoConfig(config.plugins));
```

### 🤖 Actions

Actions are a set of commands invoked by a program in order to manipulate an SVG document or pull information from it.

```js
import { Actor } from "@oxvg/wasm";

const actor = new Actor(`<svg viewBox="0 0 3 3">
  <path d="M0 0h2v2H0Z"/>
  <path d="M1 1h2v2H1Z" />
</svg>`);
actor.select("path");
actor.pathIntersect();

console.log(actor.document());
```
