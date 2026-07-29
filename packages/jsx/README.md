# OXVG Powered SVG to JSX Transformations

The `@oxvg/jsx` package allows fast and effective transformations from SVG documents into JSX and React components.

## Usage

### Node

The JSX transformer follows the [SVGR](https://react-svgr.com/docs/options/) configuration options, with some minor differences.

```ts
import { transform } from "@oxvg/jsx";

const svgCode = `
<svg xmlns="http://www.w3.org/2000/svg"
  xmlns:xlink="http://www.w3.org/1999/xlink">
  <rect x="10" y="10" height="100" width="100"
    style="stroke:#ff0000; fill: #0000ff"/>
</svg>
`

const jsCode = transform(
  svgCode,
  { icon: true },
  { componentName: 'MyComponent' },
)
```

Transforms also accept templates for building your own bespoke components.

```ts
import { transform } from "@oxvg/jsx";

const svgCode = `
<svg xmlns="http://www.w3.org/2000/svg"
  xmlns:xlink="http://www.w3.org/1999/xlink">
  <rect x="10" y="10" height="100" width="100"
    style="stroke:#ff0000; fill: #0000ff"/>
</svg>
`

const jsCode = transform(
  svgCode,
  { template: ({ imports, interfaces, componentName, props, jsx, exports }) => `
${imports}
import PropTypes from 'prop-types';
${interfaces}

function ${componentName}(${props}) {
  return ${jsx};
}

${componentName}.propTypes = {
  title: PropTypes.string,
};

${exports}
` },
)
```

### CLI

CLI usage is not available on NPM, please install the `oxvg` binary from `cargo` instead.

The binary includes the `oxvg jsx` subcommand.

## Differences from SVGR

All features available in SVGR are available here, with exceptions to the following.

- Template chunks are given as strings instead of Babel objects.
- Templates should return a string instead of Babel objects.
  - The following config option is omitted: `jsx`.
- The `svgo_config` option is replaced with `oxvg_config`. For migrating your config, use `convertSvgoConfig` from `@oxvg/napi`.
- The `svgo` option aliases `oxvg`, and will use OXVG for optimisation instead of SVGO.
- `@oxvg/jsx` uses native NAPI bindings, and is therefore only usable in native (i.e. non-web) environments.
- `@oxvg/jsx` does not accept plugins, please consider processing the input/output manually instead.
  - The following config option is omitted: `plugins`.
- `@oxvg/jsx` does not use the filesystem, please consider reading and writing files manually instead.
  - The following config options are omitted: `runtimeConfig`, `configFile`, `indexTemplate`, `index`.
