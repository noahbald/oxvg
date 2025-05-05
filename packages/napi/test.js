const { test, describe } = require("node:test");
const assert = require("node:assert");

const {
	optimise,
	Extends,
	RemoveAttrs,
	PreservePattern,
} = require("./index.js");

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

test("optimise with config", () => {
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
		Extends.None,
	);

	assert.equal(
		result,
		`<svg xmlns="http://www.w3.org/2000/svg"><!-- Should convert to currentColor --><g color="currentColor"/><g color="currentColor"/><g color="none"/><path fill="currentColor"/><path fill="currentColor"/><path fill="currentColor"/><path fill="none"/></svg>`,
	);
});

test.describe("options requiring constructors", () => {
	// FIXME: callback times out
	test.skip("prefixIds", () => {
		/** @param info {import("./index.js").PrefixGeneratorInfo} */
		const generator = (info) => {
			console.log({ info });
			return info?.name;
		};
		const result = optimise(
			`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1">
	<rect id="id" x="10" y="10" width="100" height="100" />
</svg>`,
			{
				prefixIds: {
					delim: "-",
					prefixIds: true,
					prefixClassNames: false,
					prefix: {
						type: "Generator",
						field0: generator,
					},
				},
			},
			Extends.None,
		);

		assert.equal(
			result,
			`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><rect id="id" x="10" y="10" width="100" height="100"/></svg>`,
		);
	});

	test("removeAttrs", () => {
		const result = optimise(
			`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1">
	<path fill="red" d=""/>
</svg>`,
			{
				removeAttrs: new RemoveAttrs(["path:fill"], ":", true),
			},
			Extends.None,
		);

		assert.equal(
			result,
			`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><path d=""/></svg>`,
		);
	});

	test("removeComments", () => {
		const result = optimise(
			`<svg><!-- foo --><!-- bar --></svg>`,
			{
				removeComments: { preservePatterns: [new PreservePattern("^\\s+foo")] },
			},
			Extends.None,
		);

		assert.equal(result, `<svg><!-- foo --></svg>`);
	});
});
