# Oxidised Vector Graphics for NAPI

OXVG is an effort to create high-performance SVG tooling.

It's planned to include transforming, optimising, and linting, all written in Rust.

See the main [readme](https://github.com/noahbald/oxvg/blob/main/readme.md) for more!

## Tools

The following are available through NAPI bindings

### 🪶 Optimiser

An SVG optimiser similar to [SVGO](https://github.com/svg/svgo).

# Examples

Optimise svg with the default configuration

```js
import { optimise } from "@oxvg/napi";

const result = optimise(`<svg />`);
```

Or, provide your own config

```js
import { optimise } from "@oxvg/napi";

// Only optimise path data
const result = optimise(`<svg />`, { convertPathData: {} });
```

Or, extend a preset

```js
import { optimise, extend, Extends } from "@oxvg/napi";

const result = optimise(
    `<svg />`,
    extend(Extends.Default, { convertPathData: { removeUseless: false } }),
);
```

You can even make use of your existing SVGO config

```js
import { optimise, convertSvgoConfig } from "@oxvg/napi";
import { config } from "./svgo.config.js";

const result = optimise(
    `<svg />`,
    convertSvgoConfig(config.plugins),
)
```
