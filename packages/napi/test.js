const { test, describe } = require("node:test");
const assert = require("node:assert");

const { optimise, convertSvgoConfig, extend, Actor } = require("./index.js");

// Force stable keys for snapshot objects
const snapshotSerialize = (data) =>
  JSON.stringify(
    data,
    (_, value) => {
      if (Array.isArray(value)) {
        return value
          .map((value) => JSON.stringify(value))
          .sort()
          .map((value) => JSON.parse(value));
      }
      return typeof value === "object" &&
        value !== null &&
        !Array.isArray(value)
        ? Object.fromEntries(Object.entries(value))
        : value;
    },
    2,
  );
const snapshotOptions = { serializers: [snapshotSerialize] };

test("optimise basic svg", () => {
  const result = optimise(
    `<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should convert to hex -->
    <g color="black"/>
    <g color="BLACK"/>
    <path fill="rgb(64 64 64)"/>
    <path fill="rgb(64, 64, 64)"/>
    <path fill="rgb(86.27451%,86.666667%,87.058824%)"/>
    <path fill="rgb(-255,100,500)"/>
</svg>`,
  );

  // NOTE: All features are useless, so removed
  assert.equal(result, `<svg xmlns="http://www.w3.org/2000/svg"/>`);
});

describe("optimise", async () => {
  test("optimise with config", async () => {
    const result = optimise(
      `<svg xmlns="http://www.w3.org/2000/svg">
	    <!-- Should convert to currentColor -->
	    <g color="black"/>
	    <g color="BLACK"/>
	    <g color="none"/>
	    <path fill="rgb(64, 64, 64)"/>
	    <path fill="rgb(86.27451%,86.666667%,87.058824%)"/>
	    <path fill="rgb(-255,100,500)"/>
	    <path fill="none"/>
	</svg>`,
      { convertColors: { method: "CurrentColor" } },
    );

    assert.equal(
      result,
      `<svg xmlns="http://www.w3.org/2000/svg"><!-- Should convert to currentColor --><g color="currentColor"/><g color="currentColor"/><g color="none"/><path fill="currentColor"/><path fill="currentColor"/><path fill="currentColor"/><path fill="none"/></svg>`,
    );
  });

  test("optimise with pretty indent spaces", () => {
    const result = optimise(
      `<svg xmlns="http://www.w3.org/2000/svg"><g><rect width="10" height="10"/></g></svg>`,
      extend("None"),
      { indent: 4 },
    );

    assert.equal(
      result,
      `<svg xmlns="http://www.w3.org/2000/svg">\n    <g>\n        <rect width="10" height="10"/>\n    </g>\n</svg>\n`,
    );
  });

  test("optimise with pretty indent tabs and none", () => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg"><g><rect width="10" height="10"/></g></svg>`;
    const tabsResult = optimise(svg, null, { indent: "Tabs" });
    assert.equal(
      tabsResult,
      `<svg xmlns="http://www.w3.org/2000/svg">\n\t<path d="M0 0h10v10H0Z"/>\n</svg>\n`,
    );

    const noneResult = optimise(tabsResult, null, { indent: "None" });
    assert.equal(
      noneResult,
      `<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0h10v10H0Z"/></svg>`,
    );
  });

  test("optimise with serialize options", () => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg"><g><rect style="color: rgb(255, 0, 0)" width="10" height="10"/><text>  hello  </text></g></svg>`;
    const result = optimise(svg, extend("None"), {
      indent: 2,
      attributesIndent: 2,
      useSingleQuote: true,
      enableSelfClosing: false,
      trimWhitespace: "Never",
      minify: false,
    });

    assert.equal(
      result,
      `<svg\n  xmlns='http://www.w3.org/2000/svg'>\n  <g>\n    <rect\n      style='color: red'\n      width='10'\n      height='10'>\n    </rect>\n    <text>  hello  </text>\n  </g>\n</svg>\n`,
    );

    const minified = optimise(svg, extend("None"), { minify: true });
    assert.equal(
      minified,
      `<svg xmlns="http://www.w3.org/2000/svg"><g><rect style="color:red" width="10" height="10"/><text>hello</text></g></svg>`,
    );

    assert.throws(() =>
      optimise(`<svg/>`, null, {
        // @ts-expect-error
        indent: -1,
      }),
    );
  });

  await test("optimise with invalid config", () => {
    assert.throws(
      () =>
        optimise(
          `<svg xmlns="http://www.w3.org/2000/svg">
	    <!-- Should convert to currentColor -->
	    <g color="black"/>
	    <g color="BLACK"/>
	    <g color="none"/>
	    <path fill="rgb(64, 64, 64)"/>
	    <path fill="rgb(86.27451%,86.666667%,87.058824%)"/>
	    <path fill="rgb(-255,100,500)"/>
	    <path fill="none"/>
	</svg>`,
          // @ts-expect-error
          { unknown: { method: { type: "CurrentColor" } } },
        ),
      { message: "Detected invalid optimise key: unknown" },
    );
  });

  await describe("convertSvgoConfig", () => {
    test("nullish falls back to default", () => {
      const result = convertSvgoConfig();
      assert.deepEqual(result, extend("Default"));
    });

    test("empty", () => {
      const result = convertSvgoConfig([]);
      assert.deepEqual(result, extend("None"));
    });

    test("without parameters", () => {
      const result = convertSvgoConfig([{ name: "inlineStyles" }]);
      /** @type {import("./index.d.ts").Jobs} */
      const expected = {
        inlineStyles: {
          onlyMatchedOnce: true,
          removeMatchedSelectors: true,
          useMqs: ["", "screen"],
          usePseudos: [""],
        },
      };
      assert.deepEqual(result, expected);
    });

    test("with parameters", () => {
      const result = convertSvgoConfig([
        {
          name: "inlineStyles",
          params: {
            onlyMatchedOnce: false,
            // FIXME: Once napi allows optional object values, like `#[serde(default = "...")]`
            removeMatchedSelectors: true,
            useMqs: ["", "screen"],
            usePseudos: [""],
          },
        },
      ]);
      /** @type {import("./index.d.ts").Jobs} */
      const expected = {
        inlineStyles: {
          onlyMatchedOnce: false,
          removeMatchedSelectors: true,
          useMqs: ["", "screen"],
          usePseudos: [""],
        },
      };
      assert.deepEqual(result, expected);
    });

    test("convertPathData", () => {
      const result = convertSvgoConfig([
        {
          name: "convertPathData",
          params: {
            applyTransforms: true,
            applyTransformsStroked: true,
            straightCurves: true,
            convertToQ: true,
            lineShorthands: true,
            convertToZ: true,
            curveSmoothShorthands: true,
            floatPrecision: 2,
            transformPrecision: 2,
            smartArcRounding: true,
            removeUseless: true,
            collapseRepeated: true,
            utilizeAbsolute: true,
            negativeExtraSpace: true,
            forceAbsolutePath: false,
          },
        },
      ]);
      /** @type {import("./index.d.ts").Jobs} */
      const expected = {
        // Should apply when `applyTransforms: true`
        applyTransforms: {
          applyTransformsStroked: true,
          transformPrecision: 2,
        },
        convertPathData: {
          arcCurves: false,
          closeSegments: true,
          joinNodes: true,
          removeCloseLine: false,
          removeEmptySegments: true,
          removeZeroSegments: true,
          smartArcRounding: true,
          straightCurves: true,
          tolerance: {
            angular: 0.001,
            positional: 0.001,
            precision: 2,
          },
          uniteSegments: false,
        },
      };
      assert.deepEqual(result, expected);
    });
  });
});

describe("actor", async () => {
  await test("constructor", ({ assert }) => {
    const actor = new Actor(`<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should convert to hex -->
    <g color="black"/>
    <g color="BLACK"/>
    <path fill="rgb(64 64 64)"/>
    <path fill="rgb(64, 64, 64)"/>
    <path fill="rgb(86.27451%,86.666667%,87.058824%)"/>
    <path fill="rgb(-255,100,500)"/>
</svg>`);
    assert.snapshot(actor.deriveState(), snapshotOptions);
  });

  await test("select", ({ assert }) => {
    const actor = new Actor(`<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should convert to hex -->
    <g class="different-type different-value" color="black"/>
    <g class="different-value" color="blue"/>
    <path class="different-type same-value" fill="rgb(64 64 64)"/>
    <path class="same-value" fill="rgb(64, 64, 64)"/>
    <path fill="rgb(86.27451%,86.666667%,87.058824%)"/>
</svg>`);
    actor.select("path");
    assert.snapshot(actor.deriveState(), snapshotOptions);

    actor.select(".different-value");
    assert.snapshot(actor.deriveState(), snapshotOptions);

    assert.snapshot(actor.document());
  });

  await test("document with options", () => {
    const actor = new Actor(
      `<svg xmlns="http://www.w3.org/2000/svg"><g><rect width="10" height="10"/></g></svg>`,
    );
    assert.equal(
      actor.document({ indent: 2 }),
      `<svg xmlns="http://www.w3.org/2000/svg">\n  <g>\n    <rect width="10" height="10"/>\n  </g>\n</svg>\n`,
    );
  });
});
