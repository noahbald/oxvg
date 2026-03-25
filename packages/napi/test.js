const { test, describe } = require("node:test");
const assert = require("node:assert");

const {
	optimise,
	convertSvgoConfig,
	extend,
	Actor,
} = require("./index.js");

// Force stable keys for snapshot objects
const snapshotSerialize = (data) => JSON.stringify(
	data,
	(_, value) => {
		if (Array.isArray(value)) {
			return value.map((value) => JSON.stringify(value)).sort().map((value) => JSON.parse(value))
		}
		return typeof value === "object" && value !== null && !Array.isArray(value)
			? Object.fromEntries(Object.entries(value))
			: value
		},
	2,
);
const snapshotOptions = { serializers: [snapshotSerialize]}

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
		{ convertColors: { method: { type: "CurrentColor" } } },
	);

	assert.equal(
		result,
		`<svg xmlns="http://www.w3.org/2000/svg"><!-- Should convert to currentColor --><g color="currentColor"/><g color="currentColor"/><g color="none"/><path fill="currentColor"/><path fill="currentColor"/><path fill="currentColor"/><path fill="none"/></svg>`,
	);

	await describe("convertSvgoConfig", () => {
		test("nullish falls back to default", () => {
			const result = convertSvgoConfig();
			assert.deepEqual(result, extend({ type: "Default" }));
		});

		test("empty", () => {
			const result = convertSvgoConfig([]);
			assert.deepEqual(result, extend({ type: "None" }));
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
</svg>`)
		assert.snapshot(actor.deriveState(), snapshotOptions)
	})

	await test("select", ({ assert }) => {
		const actor = new Actor(`<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should convert to hex -->
    <g class="different-type different-value" color="black"/>
    <g class="different-value" color="blue"/>
    <path class="different-type same-value" fill="rgb(64 64 64)"/>
    <path class="same-value" fill="rgb(64, 64, 64)"/>
    <path fill="rgb(86.27451%,86.666667%,87.058824%)"/>
</svg>`)
		actor.select("path")
		assert.snapshot(actor.deriveState(), snapshotOptions)

		actor.select(".different-value")
		assert.snapshot(actor.deriveState(), snapshotOptions)

		assert.snapshot(actor.document())
	})
})
